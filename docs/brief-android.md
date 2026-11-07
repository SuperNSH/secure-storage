# Brief: Android backend

Read `docs/brief-common.md`, `docs/decisions.md`, `crates/nsh-secure-storage/src/backend.rs` and `src/apple/mod.rs` first, then R2, R3, R4, R5, R6, R9, R10, R11, R14 to R18, R22, R26, R27, R31, R32 in `docs/requirements.md`, and upstream's `lib/options/android_options.dart`. You own `crates/nsh-secure-storage/src/android/**`, `tests/android*.rs`, `native/android/**`, and small additions to `src/lib.rs`, `src/options.rs`, `Cargo.toml`, and a section of `docs/decisions.md`.

## Design, already decided

No Tink, no EncryptedSharedPreferences, no RSA wrapping, no Kotlin start-up code. All of it is Rust over JNI, using `jni` at its newest version. NSH's own `jni` 0.21 is a separate copy, which is fine.

- **Reaching the VM.** NSH keeps its `JavaVM` private, and `nsh package` apps have no Kotlin of ours. Get the VM by `dlopen("libnativehelper.so")` and `dlsym("JNI_GetCreatedJavaVMs")`, once, cached. Attach on the blocking thread. Framework classes (`java.security.KeyStore`, `javax.crypto.*`, `android.security.keystore.*`) resolve through the system class loader, so threads made by Rust can find them. Verify this on a device. If it fails, say so and report what you tried.
- **Reaching the app's folder.** `android.app.ActivityThread.currentApplication()`, then `getNoBackupFilesDir()`.
- **Keys.** One AES-256-GCM key per store in `AndroidKeyStore`, alias from `storage_namespace`. Fresh random IV per value, chosen by the Keystore. Add the store and key name as AAD, so an entry moved to another key fails its tag.
- **Files.** One file per store in `no_backup/nsh-secure-storage/`, written to a temp file and renamed. Keys inside are encoded, so any key name is safe. `no_backup` is why Auto Backup and device transfer cannot restore ciphertext without its key (R22). If a file exists and the key does not, return a coded error (`ERR_SECURE_STORAGE_KEY_INVALIDATED`, or a new code you add) and keep the file.
- **Errors.** A transient Keystore failure is `_BACKEND` or `_LOCKED`, and never regenerates a key (R2, R4). Only `KeyPermanentlyInvalidatedException` or a failed AEAD tag counts as permanent, and only `reset_on_error` may then delete. The core owns the at-most-once rule, which another agent is writing, so call into it. Check its state in `src/store.rs` when you get there.
- **Options.** `AndroidOptions` already has the names. Add `require_strong_box` for upstream #655, using `setIsStrongBoxBacked`, with a coded error if the device has none. Skip upstream's cipher options, because we have one cipher, and say so.
- **Biometrics.** `android.hardware.biometrics.BiometricPrompt` (API 28+, which is NSH's minimum) needs a subclass of an abstract callback, and JNI cannot define one. Write a small Java class in `native/android/`, compile it with `d8` from the SDK into a `classes.dex` that is checked in and embedded by `include_bytes!`, and load it at run time with `dalvik.system.InMemoryDexClassLoader`. `native/android/build-dex.sh` rebuilds it and must give identical output. Its callback calls back through `RegisterNatives`. Requirements: R5 (`require_biometrics_per_operation` holds always), R6 (invalidated key gives a code, with opt-in recovery), R27 (each call resolves exactly once, including cancel, negative button and activity loss), and it never hangs.

## Testing

Split the backend so its logic runs on any host: a small trait over key operations, with the real JNI version behind `#[cfg(target_os = "android")]` and a fake in tests. Then these run in `cargo test` on this Mac: `a_transient_keystore_error_never_replaces_the_key`, `a_locked_keystore_is_retryable`, `an_invalidated_key_has_a_code_and_keeps_the_file`, `reset_on_error_recovers_once_when_opted_in`, `ciphertext_moved_to_another_key_fails_its_tag`, `file_written_atomically`, `delete_all_touches_only_its_own_store`, `a_biometric_call_resolves_exactly_once`, and the names in the requirements.

Then on a real emulator. Make your own: `avdmanager create avd -n nsh-secure -k "system-images;android-37.0;google_apis_playstore_ps16k;arm64-v8a" --device pixel_6`, and `emulator -avd nsh-secure -port 5580 -no-window -no-audio`. Never touch `emulator-5556`. Do not download other system images. API 37 is what we can test, so state that API 28 to 36 are untested. Use `ANDROID_SERIAL=emulator-5580`. Check whether `nsh-platform-android` builds from crates.io. If not, use the `.cargo/config.toml` patch that `docs/brief-apple-gaps.md` describes.

A demo app writes, reads, lists and deletes through a UI. Then: kill and reopen the app, and values remain. Check with `adb shell run-as <pkg> ls files/../no_backup/nsh-secure-storage` that the file has no plaintext, and `grep` it for the test value. For biometrics: `adb shell locksettings set-pin 1234`, enroll through Settings with `adb emu finger touch 1`, then match, fail with `finger touch 2`, cancel, and change enrollment to invalidate the key. If a step cannot be driven, say exactly what you tried. R32: check `.so` page alignment for 16 KB, with `llvm-readelf -l`.

## Finish conditions

1. `cargo fmt --all -- --check`, `cargo clippy -p nsh-secure-storage --all-targets --all-features -- -D warnings`, and `timeout 300 cargo test -p nsh-secure-storage --all-features` exit 0.
2. `cargo clippy -p nsh-secure-storage --target aarch64-linux-android --all-features -- -D warnings` exits 0.
3. Real emulator output for write, read, list, delete, restart, and no plaintext on disk.
4. Biometric results on the emulator, or exactly what could not be driven.
5. You broke the code on purpose once per group (key handling, reset rule, biometric once-only), saw the test fail and restored it.
6. Newest stable dependency versions, checked on crates.io.
