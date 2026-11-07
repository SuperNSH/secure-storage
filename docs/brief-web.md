# Brief: web backend

Read `docs/brief-common.md`, `docs/decisions.md` and `crates/nsh-secure-storage/src/backend.rs` first, then R1, R10, R11, R14 to R18, R23, R29, R30, R31 in `docs/requirements.md`, upstream's `flutter_secure_storage_web`, and `~/fo/.refs/nsh-v0.4.0/website/docs/web.md`. You own `src/web/**`, `tests/web*`, and small additions to `src/lib.rs`, `src/options.rs`, `Cargo.toml`, and a section of `docs/decisions.md`.

## Design, already decided

Behind `#[cfg(target_arch = "wasm32")]`, using `wasm-bindgen`, `wasm-bindgen-futures`, `js-sys` and `web-sys` at their newest versions, as NSH's docs describe. There are no threads and no `nsh_core::blocking` here.

- One AES-256-GCM `CryptoKey` per store, created with `extractable: false`, kept in IndexedDB. Values are encrypted with a fresh random IV, with the store and key name as AAD, and also kept in IndexedDB, not `localStorage`. This is stricter than upstream, which exports the key to `localStorage`.
- R1: first writes running at once must produce one key. Create the key inside one IndexedDB `readwrite` transaction that checks first, or put with a check that keeps the first. Our core's per-store lock is not enough, because two tabs share IndexedDB. Test with two contexts if you can.
- R29: an entry that cannot be decrypted reads as `None`, and is reported through a log line with no value. Never throw on read.
- R30: no `crypto.subtle` (an insecure origin) gives `ERR_SECURE_STORAGE_NOT_AVAILABLE` with a message saying a secure context is required.
- `WebOptions`: keep upstream's names where they mean something here. `use_session_storage` cannot apply to a key in IndexedDB, so implement "this tab only" by holding the key in memory, or mark it not supported with a coded error, and say which.
- **Honesty.** The docs must say the web has no real secret store: a script running on the same origin can use the key, though it cannot export it. This protects against reading the storage files, not against XSS.

## Testing

Run real tests in a browser: `nsh test --target web` with a small app, using `--browser` if needed. See `nsh test --help`. Write, read, list and delete through a UI, reload the page and values remain, concurrent first writes give one key, a corrupted entry reads as `None`, and IndexedDB holds no plaintext (read it from the page and search for the test value). If the web host cannot run here, try `nsh package --target web`, then serve it and drive it with the `agent-browser` skill. Say exactly what ran.

## Finish conditions

1. `cargo fmt --all -- --check`, `cargo clippy -p nsh-secure-storage --all-targets --all-features -- -D warnings`, and `timeout 300 cargo test -p nsh-secure-storage --all-features` exit 0.
2. `cargo clippy -p nsh-secure-storage --target wasm32-unknown-unknown --all-features -- -D warnings` exits 0.
3. Pasted output from a real browser for all the checks above.
4. You broke the code on purpose once per group (one-key rule, undecryptable read, secure-context check), saw the test fail and restored it.
5. Newest stable dependency versions, checked on crates.io.
