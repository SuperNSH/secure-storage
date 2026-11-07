# Platforms

What each platform's store protects, what survives uninstall, reinstall, backup and device transfer, the size limits, and what was run here. Plain statements only. Where a row says **pending**, the backend is not built or not confirmed yet, and the row will be filled in when it is.

Status as of 2026-11-07. Upstream's issues that these rows answer are listed in [`upstream-triage.md`](upstream-triage.md) (R23).

## What was run here

| Platform | What was built | What was run | What was not run |
|---|---|---|---|
| macOS, Rust crate | `cargo test` for the crate, Apple backend on a file keychain | The crate's 1 MiB round trip (`large_value_round_trips_on_every_platform`) on a throwaway file keychain: passed. The TSX package's 21 tests on the memory and host backends: passed. | The Rust store through the Native Shell runner: a save fails with `ERR_SECURE_STORAGE_BACKEND` on the same throwaway keychain (see below). |
| iOS, Rust crate | `cargo check --target aarch64-apple-ios-sim`: passes | Nothing on a simulator or device | A simulator or device run |
| iOS embedding host | `native/ios/SecureStorageHost.swift`, `swiftc -typecheck` against the iPhone Simulator SDK: passes | Nothing in a host app | A host app run |
| Android embedding host | `native/android/SecureStorageHost.kt`, compiled with the Kotlin 2 compiler against `android-37.0`: passes | Nothing on a device or emulator | A device or emulator run |
| Android, Rust crate | **pending** | **pending** | **pending** |
| Linux | **pending** | **pending** | **pending** |
| Windows | **pending** | **pending**. There is no Windows machine here. | Everything |
| Web | **pending** | **pending** | **pending** |

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

**Pending.** The Android backend is being built in `crates/nsh-secure-storage` by another part of the kit. This section will describe AndroidKeyStore, the `no_backup` folder and the uninstall, backup and transfer behavior once it is built and run on a device or emulator.

## Linux

**Pending.** Planned on `keyring-core` with the Secret Service (for example gnome-keyring). The Secret Service must be running and unlocked, and a locked or missing keyring must be reported as an error, not as an absent value. Survives uninstall: Secret Service items remain until deleted. This section will be confirmed when the backend is built and run.

## Windows

**Pending.** Planned on `keyring-core` with the Windows Credential Manager. Each app gets its own Credential Manager target. Survives uninstall: Credential Manager entries stay until deleted. Not backed up: values are per machine. **Not run here**, because there is no Windows machine. The row will stay pending until a Windows machine runs the tests.

## Web

**Pending.** The web has no secret store. The planned backend stores a non-extractable WebCrypto key in IndexedDB, and encrypts values with it. Read this plainly: **any script that runs on the page can read what the page can read, and the browser may clear site storage at any time.** The key being non-extractable does not change that, because the page can still call the key to decrypt. Clearing site data removes everything. Nothing is backed up. Web needs a secure context (HTTPS or localhost), and an insecure context fails with `ERR_SECURE_STORAGE_NOT_AVAILABLE`. Confirmed only when the backend is built and run.

## Limits that apply everywhere

- Keys must not be empty and must not contain a NUL character.
- An empty string is a value, distinct from an absent key.
- Errors and logs carry the code and the key's name, never a value.
- A value is never written to disk outside the platform's store, unencrypted, on any platform.
