# Brief: Windows and Linux backends

Read `docs/brief-common.md`, `docs/decisions.md` and `crates/nsh-secure-storage/src/backend.rs` first, then R2, R7, R8, R10, R11, R14 to R20, R26, R31 in `docs/requirements.md`, and upstream's Windows and Linux plugins for behavior. You own `src/windows/**`, `src/linux/**`, `src/keyring_store.rs` if you add it, `tests/windows*.rs`, `tests/linux*.rs`, and small additions to `src/lib.rs`, `src/options.rs`, `Cargo.toml`, and a section of `docs/decisions.md`.

## Design, already decided

The owner wants existing crates reused. Use `keyring-core` for both, behind our `Backend` trait.

- **Windows.** `windows-native-keyring-store` holds one AES-256 key per store in Credential Manager. Credential Manager limits value size, so values live in one AES-256-GCM file per store under `%LOCALAPPDATA%`, using `aes-gcm` and `getrandom`, with the store and key name as AAD. R19: the credential's target name includes an app identity, so two apps do not share. Find how to read the app's identity from NSH, and let an option override it. R20: write to a temp file, flush and rename, so a crash cannot leave half a file. Read upstream's option names, and keep what applies.
- **Linux.** `zbus-secret-service-keyring-store` with its `rt-tokio-crypto-rust` feature, or whichever feature fits NSH's tokio runtime. Each key is one item, with attributes for the store and the key. `read_all` and `delete_all` use `keyring-core`'s search, limited to this store. Values of 1 MiB must round-trip, or the limit is documented with a coded error (R18). R7: read a write back and compare, and report a rejected store as a coded error, not silence. R8: no Secret Service, locked keyring and refused calls are each coded. Say in the docs that a sandboxed app, such as a Flatpak, needs the talk-name permission for the Secret Service, and that this is untested.

Before writing, read `keyring-core`'s source for the API (`Entry`, `search`, `Error`), and its `mock` module. Both are on crates.io, and any newer stable version is what you use. Map every `keyring_core::Error` to our codes.

Put shared logic (the encrypted-file store, key-name encoding) where it can run on this Mac. Then test it with `keyring-core`'s `mock` store instead of a real one, in `cargo test`: `windows_values_file_round_trips`, `windows_file_is_written_atomically`, `windows_two_apps_do_not_share_a_target`, `a_tampered_file_fails_its_tag`, `a_failed_credential_lookup_never_replaces_the_key`, `long_values_round_trip`. Never use the real store on this Mac, because that is the owner's login keychain.

## Real runs

- **Linux, in Docker**, which works here. Write `tools/linux-test/Dockerfile` with Rust, D-Bus and `gnome-keyring`, running `dbus-run-session` and unlocking the keyring with a throwaway password piped to `gnome-keyring-daemon --unlock`. Run the real Secret Service tests there, behind an env var such as `NSH_SECURE_STORAGE_LINUX_TESTS=1`: round trip, read_all, delete_all scoping, special characters, 1 MiB, locked keyring gives a code, and no Secret Service gives a code. Mount the repo, and keep the build folder in a Docker volume. If `nsh-core` needs GTK, install it in the image.
- **Windows.** There is no Windows machine here. `cargo check` and `cargo clippy` for `x86_64-pc-windows-gnu` must pass. Say plainly that the real Credential Manager has not been run, and add a short `tools/windows-test.md` with the exact commands to run on Windows.

## Finish conditions

1. `cargo fmt --all -- --check`, `cargo clippy -p nsh-secure-storage --all-targets --all-features -- -D warnings`, and `timeout 300 cargo test -p nsh-secure-storage --all-features` exit 0.
2. `cargo clippy -p nsh-secure-storage --target x86_64-pc-windows-gnu --all-features -- -D warnings` and `--target x86_64-unknown-linux-gnu` exit 0.
3. Pasted output of the Linux tests in Docker.
4. You broke the code on purpose once per group (file atomicity, tag check, read-back, error mapping), saw the test fail and restored it.
5. Newest stable dependency versions, checked on crates.io.
