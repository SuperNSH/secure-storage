# Decisions

Written by the agent that builds the Rust core. Each entry says what was chosen and why.

## Existing crates versus our own backends

The brief asks whether an existing crate covers each platform. The keyring crates were read at source.

| Platform | Decision | Why |
|---|---|---|
| Apple (iOS, macOS) | Own backend on `security-framework` 3.7.0 and `security-framework-sys` 2.17.0 | `apple-native-keyring-store` uses the same library, but it has no per-item label, comment, description or invisible flags, and no operation prompt text. Upstream's options need those. |
| Windows | To build on `keyring-core` with `windows-native-keyring-store` | To be confirmed by the agent that builds Windows. The `Backend` trait fits a keyring-core store behind it. |
| Linux | To build on `keyring-core` with `zbus-secret-service-keyring-store` | To be confirmed by the agent that builds Linux. The `Backend` trait fits a keyring-core store behind it. |
| Android | Own JNI backend (AndroidKeyStore AES-GCM, `no_backup` folder) | `android-native-keyring-store` keeps data in SharedPreferences, which Auto Backup includes by default. It has no biometric prompt, and it needs Kotlin start-up code. |
| Web | Own backend (non-extractable WebCrypto key in IndexedDB) | No secret store exists on the web, so no crate changes what the page can read. |

The `Backend` trait in `src/backend.rs` is the seam. It stays as it is for the other platforms.

## Store handle and listeners

- `SecureStorage` is a `Copy` handle to a small allocation that is kept for the life of the process. Make one handle per store.
- Listeners belong to a handle, so two engines do not share a listener map.
- Two handles on one store share a lock, found by the store's identity (the backend's `store_id`), so their writes run in order.

## Ordering: writes wait, reads wait for earlier writes

- Writes and deletes hold the store's lock while they run.
- A read waits for the writes queued before it, then runs without the lock. A write issued during a slow read lands at once.
- The reactive read drops an answer that started before a write or refresh (its revision check). This is the rule "a write made during a read wins". With a lock held for reads too, the rule could never be reached, so reads do not hold it.
- Break test: removing the revision check fails `a_write_during_a_slow_read_wins`.

## Keys and values

- A key must not be empty and must not contain NUL. Other characters are allowed.
- Empty strings and long values are values. `write(key, None)` deletes the key, as upstream does.
- Values are held in `Zeroizing<String>`. The `Debug` output of `Zeroizing` in zeroize 1.9.1 prints no contents, and no library type stores a value in `Debug`.

## Upstream differences

- `reset_on_error` defaults to `false` on Android (upstream defaults to `true`). A silent reset loses the user's data. Only an explicit, permanent signal can reset, and only when the caller asks for it.
- `read_all` on Apple returns at most `result_limit` items. `None` returns all of them.
- `use_secure_enclave` is accepted, but not built. A call that sets it fails with `ERR_SECURE_STORAGE_NOT_AVAILABLE`. It never falls back to a weaker mode in silence.
- `should_return_persistent_reference` is accepted and has no effect, since persistent references are not exposed.
- `creation_date` and `last_modified_date` are accepted and not sent: the keychain sets them.
- `operation_prompt` is an option of its own, for the prompt text (`kSecUseOperationPrompt`).

## Apple: what each option does

- Items are generic passwords. The service is `account_name` (default `nsh_secure_storage`). `read_all` and `delete_all` touch only that service.
- Searches match synchronizable and local items (on the data protection keychain), and any accessibility level. An item written at one level is found by a read that names another (upstream #1269).
- A write updates the item in place when one exists, and adds it otherwise. A failed update never removes the item. Changing the access control of an existing item is not done by an update: delete the item first.
- `AuthenticationUi::Fail` and `Skip` never open a dialog. For a file keychain the system ignores `kSecUseAuthenticationUI` and still asks, so the library turns keychain interaction off for the length of the call (a counted guard, restored when the last call ends). It also checks the keychain's lock state first, and a locked file keychain returns `ERR_SECURE_STORAGE_LOCKED`.
- `AuthenticationUi::Allow` and an unset option may show the system's own prompt. That is the platform's normal unlock flow for the app's own items.
- The file keychain (macOS, `uses_data_protection_keychain: false`) has these limits: access control is refused up front (`ERR_SECURE_STORAGE_NOT_AVAILABLE`, since the keychain returns `errSecParam`), no iCloud sync, and `read_all` and `delete_all` act one item at a time (one query cannot read attributes and data together, and one delete can leave matches behind).
- The data protection keychain on macOS needs the keychain access group entitlement. An unsigned binary gets `ERR_SECURE_STORAGE_MISSING_ENTITLEMENT`, with a message that says what to add.
- `security-framework-sys` 2.17.0 does not export some Security constants (`kSecAttrAccessible`, `kSecAttrIsInvisible`, `kSecAttrIsNegative`, `kSecMatchLimitOne`, `kSecUseOperationPrompt`, the two auth-UI values, and `kSecUseDataProtectionKeychain` on macOS). They are declared in `src/apple/sys.rs`, with the same names as the SDK.
- The status-code mapping is in `src/apple/status.rs`. Break test: changing `errSecInteractionNotAllowed` to `Backend` fails `macos_locked_keychain_gives_locked`.

## Test process: no system dialogs

- A test run must never open a system dialog on the owner's machine. `tests/apple.rs` turns keychain interaction off for the whole test process, once, before any keychain call. It also removes leftover `nsh-secure-storage-*.keychain-db` files from earlier runs.
- Each Keychain test uses a throwaway keychain file in the temp folder, never the login keychain.
- An earlier run opened a dialog through `security lock-keychain` followed by a read. That is why the locked case now uses the library's own check, and no longer depends on the system prompt.

## Platform protection, stated plainly

- Apple: items are protected by the Keychain, at the accessibility level given. `ThisDeviceOnly` levels do not migrate to another device.
- Web (not built yet): there is no secret store on the web, so a script that runs on the page can read what the page can read. The web backend's docs say so.

## Error codes against docs/requirements.md

Codes follow the brief: `ERR_SECURE_STORAGE_*`. The requirements file uses short names. They map as follows: `locked` is `ERR_SECURE_STORAGE_LOCKED`; `entitlement_missing` is `ERR_SECURE_STORAGE_MISSING_ENTITLEMENT`; `keychain_error` and `store_unavailable` are `ERR_SECURE_STORAGE_BACKEND`; `backend_unavailable` is `ERR_SECURE_STORAGE_BACKEND`; `key_invalidated` is `ERR_SECURE_STORAGE_KEY_INVALIDATED`; `insecure_context` (web) is `ERR_SECURE_STORAGE_NOT_AVAILABLE`.

## What each platform protects, and what survives

Apple is built and tested. The other rows describe each platform's design; the agent that builds that backend confirms its row.

| Platform | Protects against | Survives uninstall | Survives backup | Survives sync |
|---|---|---|---|---|
| Apple (iOS, macOS) | Other apps and other devices, while the device is locked, at the chosen accessibility level | iOS Keychain items are not removed by uninstall, so `deleteAll` before uninstalling | `Unlocked` and `FirstUnlock` levels follow an encrypted backup; `ThisDeviceOnly` levels never do | Only with `synchronizable: true` (iCloud Keychain) |
| Android | Other apps, and a copy of the data taken without the device's key (AES-GCM key in AndroidKeyStore) | Uninstall removes the data and the key | Values sit in `no_backup`, so backup and transfer exclude them; a restored ciphertext without its key is reported and kept | No sync |
| Windows | Other Windows users, and a copy of the values file without the Credential Manager key | Uninstall leaves Credential Manager entries in place | Not backed up (values are per machine) | No sync |
| Linux | Other users, through the Secret Service, which needs a running service such as gnome-keyring | Secret Service items remain until deleted | Depends on the keyring's own storage | No sync |
| Web | Nothing secret: the key sits in the page's own IndexedDB, so any script on the page can read what the page reads. The browser may clear site storage at any time | Clearing site data removes everything | Not backed up | No sync |

## Android: keys, the store file, and resets

- **Keys.** One AES-256-GCM key per store, in AndroidKeyStore, under the alias `nsh-secure-storage.<namespace>`. The Keystore picks each IV. The store file keeps the IV and the ciphertext (tag included) per key. No cipher code of ours runs on device.
- **Additional data.** Each entry is sealed with its store name and key name, length-prefixed. An entry moved to another key or store fails its tag (`ciphertext_moved_to_another_key_fails_its_tag`).
- **File.** One JSON file per store in `no_backup/nsh-secure-storage/`, named after the base64url namespace. Keys are base64url in the file, so any key name is safe. A write goes to a temporary file, is synced, then renamed over the old one. The file holds no plaintext.
- **Permanent signals.** Only `KeyPermanentlyInvalidatedException` and a failed AEAD tag are permanent. Only these may reset, and only when `reset_on_error` is true (default `false`, unlike upstream). A reset deletes the store file, then its key, makes one fresh key, and retries the write once. A second failure is reported as `ERR_SECURE_STORAGE_KEY_INVALIDATED`. A dedicated `reset_failed` code would need a variant in `error.rs`, which the core owns, so it is not added.
- **Transient errors.** A failed lookup of the key is an error, never "no key", so it cannot make a new key. A locked keystore is `ERR_SECURE_STORAGE_LOCKED`, and never reaches the reset path.
- **Missing key.** If the file has entries and the key does not exist (a restore brought the ciphertext back alone), reads and writes return `ERR_SECURE_STORAGE_KEY_INVALIDATED` with the key name. The file is kept, and no key is made.
- **`delete_all`** removes this store's file and key, and nothing of any other store. A store whose key was invalidated is therefore usable again after `delete_all`, without `reset_on_error`.
- **`contains_key`** checks the file for an entry and never opens it, so it never shows a biometric prompt. It agrees with `read` except when the entry cannot be opened.
- **Options.** `require_strong_box` (upstream #655) keeps the key in StrongBox. A device without one returns `ERR_SECURE_STORAGE_NOT_AVAILABLE`, with no silent fallback. Upstream's cipher options (`keyCipherAlgorithm`, `storageCipherAlgorithm`), its migration options, `preferencesKeyPrefix` and `migrateWithBackup` are not offered: there is one cipher, and NSH has no legacy data to migrate.
- **Dependencies.** Android only: `jni` 0.22.4, `libc` 0.2.191, `base64` 0.23.1, `serde` 1.0.229, `serde_json` 1.0.154. `aes-gcm` 0.11.1 is a dev-dependency, used only by the host fake that seals for the tests. `getrandom` and `zeroize`'s extra features are not needed.
- **Host tests.** `src/android/tests.rs` runs the store logic against a fake Keystore on any host. The Keystore itself is only on a device, so JNI and the emulator are checked separately.
