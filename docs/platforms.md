# Platforms

What each platform's store protects, what survives uninstall, reinstall, backup and device transfer, the size limits, and what was run here. Plain statements only. Everything here is checked by reading the source and by the CI runs on GitHub Actions (this repo builds and tests there, not on a developer's machine). A claim that no run backs is marked unverified.

Status as of 2026-11-19. Upstream's issues that these rows answer are listed in [`upstream-triage.md`](upstream-triage.md) (R23).

## What was run here

| Platform | What was built | What was run | What was not run |
|---|---|---|---|
| macOS, Rust crate | `cargo test` for the crate, Apple backend on a file keychain | The crate's 1 MiB round trip (`large_value_round_trips_on_every_platform`) on a throwaway file keychain: passed. The TSX package's 21 tests on the memory and host backends: passed. | The Rust store through the Native Shell runner: a save fails with `ERR_SECURE_STORAGE_BACKEND` on the same throwaway keychain (see below). |
| iOS, Rust crate | `cargo check --target aarch64-apple-ios-sim`: passes | Nothing on a simulator or device | A simulator or device run |
| iOS embedding host | `native/ios/SecureStorageHost.swift`, `swiftc -typecheck` against the iPhone Simulator SDK: passes | Nothing in a host app | A host app run |
| Android embedding host | `native/android/SecureStorageHost.kt`, compiled with the Kotlin 2 compiler against `android-37.0`: passes | Nothing on a device or emulator | A device or emulator run |
| Android, Rust crate | Store logic in `cargo test` on every host, against a fake Keystore that really seals with AES-GCM (`src/android/tests.rs`). `cargo clippy --target aarch64-linux-android` is a CI job | The JNI Keystore on a device or emulator: not run in this audit | A device or emulator run, and every biometric prompt |
| Linux | `cargo test` against `keyring-core`'s mock store on every host. The real Secret Service tests (`tests/linux_secret_service.rs`, `linux_locked_keyring.rs`, `linux_no_service.rs`) run in the CI `rust (ubuntu-latest)` job under `dbus-run-session` and gnome-keyring | Round trip, scoped `read_all` and `delete_all`, special characters, 1 MiB, a locked keyring and no service, on that runner | Other keyrings (KWallet, KeePassXC), and a Flatpak or Snap sandbox |
| Windows | `cargo test` against `keyring-core`'s mock store (`tests/windows.rs`), and the CI `rust (windows-latest)` job | The mock-store tests on a Windows runner | The real Credential Manager: no test writes to it (see `tools/windows-test.md`) |
| Web | `cargo clippy --target wasm32-unknown-unknown` is a CI job. The browser app in `tests/web_app` | `nsh test --target web` on `tests/web_app`: write, read, list, contains, delete, empty value, several writes at once, delete all, a NUL key, and a reload | Two tabs racing first writes, a corrupted entry, an insecure context, and the IndexedDB bytes read for plaintext: no test does these |

The TSX package's tests use a memory store and a fake host (the `secureStorage.*` handlers are written in the test). They prove the TypeScript behavior, not any platform's store.

## Open item: the Rust store through the runner on macOS

`nsh test` on the demo app (`examples/demo-rs`) builds the crate into the runner and saves a value into a throwaway file keychain at `/private/tmp/nsh-secure-storage-demo-rs.keychain-db`. The save fails with `ERR_SECURE_STORAGE_BACKEND`. A Rust test that runs the same calls on the same file, outside the runner (delete, write, read), passes. The cause is not known. Until it is, the macOS run through the runner is not confirmed, and the real `nsh test --target macos` run was not done.

A related finding: with a keychain path that does not exist, the crate's write reported success and the read found nothing. The item may have been written somewhere else. Use only a keychain file that exists, and make it with `security create-keychain -p "" <absolute path>`.

## Apple: macOS and iOS (Keychain)

Items are generic passwords in the Keychain, filed under the store's account name (default `nsh_secure_storage`). The default accessibility is `unlockedThisDevice`.

- **Protects against:** other apps, which cannot read the item (the Keychain returns it only to the app that wrote it, or to the access group it names), and a copy of the device's storage taken while the device is locked. Each item is readable only at the accessibility level it was written with.
- **Does not protect against:** code running inside the app, a person who unlocks the device, or a jailbroken device.
- **Survives uninstall:** iOS Keychain items are not deleted with the app. Call `deleteAll()` before uninstalling if the data should go.
- **Survives reinstall:** yes, if the app is reinstalled on the same device, the items are still there.
- **Survives backup:** `unlocked`, `firstUnlock` and the other levels without `ThisDevice` follow an encrypted backup. The `ThisDeviceOnly` levels never do.
- **Survives device transfer:** only through an encrypted backup restore, and only for the levels above. `ThisDeviceOnly` items stay on the old device.
- **Sync:** only with `synchronizable: true`, which puts the item in iCloud Keychain.
- **Size limits:** a 1 MiB value round-trips on the file keychain (tested here). Larger values, and the Keychain's own item limit, were not measured.
- **Not built:** access control flags (biometrics, passcode) are in the Rust crate. The iOS embedding host refuses them rather than ignoring them.

## Apple embedding host (`native/ios/SecureStorageHost.swift`)

The same Keychain, used by a host app that embeds NSH. It implements the `secureStorage.*` handlers with Security calls.

- **Protects against:** as above.
- **Writes** update the item in place, and add it only when it is missing. A failed update never removes the item.
- **Reads** never show a prompt: a read that needs one fails with `ERR_SECURE_STORAGE_LOCKED`.
- **Refuses**, with `ERR_SECURE_STORAGE_NOT_AVAILABLE`, the options it does not build: access control flags, access groups, and an authentication UI other than `fail` or `skip`.
- **Survives uninstall, reinstall, backup and transfer:** as the Keychain row above.
- **Tested here:** typecheck only.

## Android embedding host (`native/android/SecureStorageHost.kt`)

Values are AES-256/GCM ciphertext, under a key in AndroidKeyStore that is created once and never leaves the device's key store. The ciphertext and its IV sit in one file in the app's `no_backup` folder, written whole through a temporary file and renamed.

- **Protects against:** other apps, and a copy of the app's files taken without the device's key. Without the key the ciphertext is useless.
- **Does not protect against:** code running inside the app, or a device whose key store is compromised.
- **Survives uninstall:** no. Uninstalling removes the file, and the key goes with the app's key entries.
- **Survives reinstall:** no. The key goes with the app, so values written before the reinstall are gone.
- **Survives backup and transfer:** no. The `no_backup` folder is excluded from Auto Backup and device transfer, so no ciphertext is restored without its key.
- **Failures:** a failed AES-GCM tag or an invalidated key is reported as `ERR_SECURE_STORAGE_CORRUPT` or `ERR_SECURE_STORAGE_KEY_INVALIDATED`, and the value is kept. The value is removed only when the caller set `aOptions.resetOnError: true`. The default is `false`.
- **Size limits:** not measured here.
- **Tested here:** Kotlin compile against `android-37.0`. Nothing else.

## Android, Rust crate

Values are AES-256-GCM ciphertext under one key per store in AndroidKeyStore (alias `nsh-secure-storage.<namespace>`). The IV and ciphertext of each value sit in one JSON file in the app's `no_backup/nsh-secure-storage/` folder, written to a temporary file, synced and renamed. Each value is sealed with its store and key name as additional data, so an entry moved to another key fails its tag. All the AES runs in the Keystore through JNI.

- **Protects against:** other apps, and a copy of the app's files taken without the device's key.
- **Does not protect against:** code inside the app, or a compromised key store. `require_strong_box` puts the key in StrongBox, and a device without one gets `ERR_SECURE_STORAGE_NOT_AVAILABLE`.
- **Survives uninstall and reinstall:** no. The file and the key go with the app. A reinstall starts empty.
- **Survives backup and transfer:** no. `no_backup` is outside Auto Backup and device transfer, so ciphertext is never restored without its key. If a file somehow comes back without its key, reads return `ERR_SECURE_STORAGE_KEY_INVALIDATED` with the key name, the file is kept, and no new key is made.
- **Errors:** a transient Keystore failure is `ERR_SECURE_STORAGE_BACKEND`, a locked keystore is `ERR_SECURE_STORAGE_LOCKED`, and neither replaces a key. Only `KeyPermanentlyInvalidatedException` or a failed AEAD tag is permanent, and only `reset_on_error: true` (default `false`) may then delete the file and key, once per call.
- **Size limits:** not measured on a device.
- **Biometrics are not usable yet.** The key is made with user authentication when `enforce_biometrics` or `require_biometrics_per_operation` is set, but the crate has no `BiometricPrompt` (it needs a Java callback class loaded as a dex, which is not written). Such a key cannot be used, and calls on it return `ERR_SECURE_STORAGE_LOCKED`. Do not set these options. Requirements R5, R6 (recovery in per-operation mode) and R27 are open for this reason.
- **Tested here:** the store logic on any host, and a CI clippy of the Android target. **Not run:** the JNI Keystore, and 16 KB page alignment of the shipped `.so` (R32), on any device or emulator in this audit.

## Linux

Each key is one Secret Service item through `keyring-core` and `zbus-secret-service-keyring-store`. The item's `service` attribute is the store name (`account_name`) and its `username` is the key. There is no libsecret schema or label to relabel, so upstream's relabel problem cannot arise.

- **Protects against:** other users, and a copy of the disk, while the keyring is locked. It needs a running Secret Service (for example gnome-keyring). Any process of the same user that can reach the unlocked service can read the items.
- **Failures:** a missing service, a locked keyring and a refused call each return a coded error, never an absent value. After a write the item is read back and compared. A store that keeps other bytes is retried once, then reported.
- **Survives uninstall:** yes, items remain until deleted. **Backup and sync:** depends on the keyring's own storage.
- **Size limits:** 1 MiB round-trips (tested on the CI runner).
- **Sandboxes:** a Flatpak app needs the `--talk-name=org.freedesktop.secrets` permission. This is untested.

## Windows

One AES-256 key per store sits in Credential Manager, through `keyring-core` and `windows-native-keyring-store`. Values are in one AES-256-GCM file per store under `%LOCALAPPDATA%`, in a folder named for the app, because Credential Manager limits value size. The app is the process's AppUserModelID (from `nsh.json`'s `id`), or `windows.app_id`. The file is written to a temporary file and renamed, a locked file is retried and then reported, and a failed write never truncates it.

- **Protects against:** other Windows users, and a copy of the values file without the Credential Manager key.
- **Does not protect against:** code running as the same user, which can read the key.
- **Survives uninstall:** Credential Manager entries and the values file stay until deleted. **Backup and transfer:** per machine, not synced.
- **Verified:** the file logic and the error mapping on `keyring-core`'s mock store, in CI on a Windows runner. **Not verified:** the real Credential Manager. The only way here is by running a test on Windows, which has not been done.

## Web

Values are AES-256-GCM sealed under one non-extractable WebCrypto key per store. The key and the values are kept in IndexedDB, not `localStorage`. Read this plainly: **the web has no real secret store. Any script that runs on the page can call the key to decrypt, though it cannot export it, and the browser may clear site storage at any time.** This protects the stored files, not the page.

- **Secure context:** the backend refuses to run without `crypto.subtle` or outside a secure context (HTTPS or localhost), with `ERR_SECURE_STORAGE_NOT_AVAILABLE`.
- **Undecryptable entries** read as absent and are logged by key name only.
- **First writes:** a new key is added with a check-and-store in one IndexedDB request, so two tabs cannot end with two keys.
- **`use_session_storage`** cannot hold for a key in IndexedDB, so it returns `ERR_SECURE_STORAGE_NOT_AVAILABLE`.
- **Survives:** a reload (tested in a browser). Clearing site data removes everything. Nothing is backed up or synced.
- **Not verified:** the two-tab race, a corrupted entry and the insecure-context error have no browser test.

## Requirement status (audit 2026-11-19)

Read from the source, not from earlier claims. "Test" is the named test of `docs/requirements.md`.

| Status | Requirements |
|---|---|
| Done, with a named test | R2, R3, R4, R7, R9, R10, R11, R12, R13, R14, R15, R16, R17, R18, R19, R20 (as `windows_file_is_written_atomically`), R21, R22, R23, R24, R25, R26, R28, R31 |
| Done in code, no browser test | R1, R29, R30 (web) |
| Done by design | R8: no libsecret schema or label exists, so `linux_schema_attribute_follows_relabel` has nothing to test. The locked, missing and refused cases are tested |
| Not done | R5, R6 in per-operation mode, R27: no `BiometricPrompt` (see Android) |
| Not verified | R32: no test or CI step reads the `.so` alignment |

Gates are CI's: the latest run on this commit shows `format`, `rust (windows-latest)`, `rust (ubuntu-latest)`, `check (wasm32-unknown-unknown)` and `check (aarch64-linux-android)` passing, with the macOS, iOS and TypeScript jobs still running when this was written.

## Limits that apply everywhere

- Keys must not be empty and must not contain a NUL character.
- An empty string is a value, distinct from an absent key.
- Errors and logs carry the code and the key's name, never a value.
- A value is never written to disk outside the platform's store, unencrypted, on any platform.
