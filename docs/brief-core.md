# Brief: Rust core, memory backend and Apple backend

Read `docs/brief-common.md` first. You own `crates/nsh-secure-storage/**`, `Cargo.toml`, `Cargo.lock` and `docs/decisions.md`. Another agent writes `docs/upstream-triage.md` and `docs/requirements.md` at the same time. If `docs/requirements.md` exists when you finish a group, read it and meet what applies to `core` and `apple`.

Other agents add the Android, Windows, Linux and web backends after you, so make that easy and do not write them. Give each platform its own module, behind `#[cfg]`, and keep each backend's dependencies under target-specific tables.

## The API

Read `~/fo/.refs/flutter_secure_storage/flutter_secure_storage/lib/flutter_secure_storage.dart`, then every file in `lib/options/`. Keep upstream's names, in snake_case:

```rust
let storage = SecureStorage::new(SecureStorageOptions::default());
storage.write(key, value, &WriteOptions::default()).await?;
storage.read(key, &ReadOptions::default()).await?;      // Option<Zeroizing<String>>
storage.contains_key(key, ..).await?;
storage.delete(key, ..).await?;
storage.read_all(..).await?;
storage.delete_all(..).await?;
storage.is_protected_data_available();                  // iOS; true elsewhere
```

- `SecureStorage` is a `Copy` handle. Calling it never blocks the engine thread.
- Writing `None` as a value deletes the key, as upstream does.
- Options: `AppleOptions`, with `IosOptions` and `MacOsOptions` (account name, group id, accessibility, synchronizable, access control flags, label, description, comment, is_invisible, is_negative, result_limit, uses_data_protection_keychain, and the rest in the upstream file), `AndroidOptions`, `WindowsOptions`, `LinuxOptions`, `WebOptions`. Each option an agent later implements goes here now, as a record with `#[non_exhaustive]` and `Default`, so later agents only read it.
- Listeners: `register_listener(key, listener)` returns a stop, plus `unregister_all_listeners_for_key` and `unregister_all_listeners`. Upstream's semantics: called after a write or delete through this storage with the new value, or `None`. Unlike upstream, listeners are per surface, so two engines do not share a global map.
- `on_protected_data_change(listener)` returns a stop, for upstream's `onCupertinoProtectedDataAvailabilityChanged`.
- `SecureStorageError`: `code()` gives a stable string, such as `ERR_SECURE_STORAGE_LOCKED`, `_NOT_AVAILABLE`, `_KEY_INVALIDATED`, `_CORRUPT`, `_AUTH_CANCELLED`, `_AUTH_FAILED`, `_MISSING_ENTITLEMENT`, `_INVALID_KEY`, `_BACKEND`, plus the platform's own status number where there is one. `Display` and `Debug` never hold a value.

**The reactive read is NSH's own addition.** `use_secure_value(storage, key)` returns a `Copy` handle with `value()`, `state()` (`Loading`, `Ready`, `Error`), `error()`, and `refresh()`. It follows writes and deletes made through that storage. A binding on one key wakes only for that key. A write made while an older read is in flight wins over that read's answer. The value is `Zeroizing`, and it is cleared when the owner goes.

**The backend seam.** A trait the other agents implement, async because the web has no threads. Native backends wrap their blocking calls in `nsh_core::blocking`. All backends get a per-store async lock from the core, so concurrent operations on one store run in order, including the very first ones. The core owns validation of keys and the listener and reactive machinery. Backends only store. Include the store's identity in the lock, so two `SecureStorage` handles and two surfaces on one store are serialized.

## Memory backend

`SecureStorage::memory()` and `with_initial_values(map)`: upstream's `setMockInitialValues`. It is what `Headless` tests use, and what any app's tests use. Headless tests must never reach a real store unless asked.

## Apple backend

For iOS and macOS, with `security-framework` at its newest version, and `security-framework-sys` for what its safe API lacks. Read `flutter_secure_storage_darwin/darwin/.../FlutterSecureStorage.swift`. Cover: service and account mapping; access group; every accessibility level; synchronizable; access control flags through `SecAccessControl`, including `biometryAny`, `biometryCurrentSet`, `userPresence`, `devicePasscode`, `watch`, and the combinators; the operation prompt text; `read_all` and `delete_all` scoped to this account name; finding items across accessibility levels (upstream #1269); updating an item when its attributes change; empty strings; long values; keys with special characters; mapping `errSecInteractionNotAllowed` to `LOCKED`, `errSecMissingEntitlement` to `MISSING_ENTITLEMENT`, `errSecUserCanceled` to `AUTH_CANCELLED`, `errSecAuthFailed` to `AUTH_FAILED`. On macOS, the file keychain is the fallback only when `uses_data_protection_keychain` is `false`; a missing entitlement is a coded error with a message that says how to fix it. A `keychain_path` option targets a keychain file, which tests use.

`is_protected_data_available` and its listener on iOS: read `UIApplication.isProtectedDataAvailable` and its notifications through `objc2-ui-kit`, at its newest version.

## Native module

`#[native_module("nsh-secure-storage/native")]` at the crate root, exposing all this to TSX as a class or functions with Promise results and `code` on failures. Read `references/mixed.md`. Another agent writes the TypeScript wrapper over it, so document each export.

## Tests

`crates/nsh-secure-storage/tests/`, through `Headless` where UI is involved. At least:

- `write_read_delete_round_trip`, `read_all_and_delete_all`, `contains_key`, `writing_none_deletes`, `empty_string_is_a_value`, `long_value_round_trips`, `keys_with_special_characters`, `delete_all_touches_only_its_own_store`
- `listener_runs_on_write_and_delete`, `stop_ends_a_listener`, `listeners_are_per_surface`
- `a_binding_on_one_key_wakes_only_for_that_key`, with run counters
- `a_write_during_a_slow_read_wins`, `concurrent_first_writes_all_land`, `two_handles_on_one_store_run_in_order`
- `value_never_reaches_debug_or_an_error`
- `use_secure_value_clears_when_its_owner_goes`
- On macOS, behind `#[cfg(target_os = "macos")]`: `macos_keychain_round_trip` and friends, against a throwaway keychain file created in a temp folder and deleted afterwards. Cover update, read_all scoping, across-accessibility find, special characters, and a locked keychain giving `LOCKED`. Never touch the login keychain.

## Finish conditions

1. `cargo fmt --all -- --check` exits 0.
2. `cargo clippy --all-targets --all-features -- -D warnings` exits 0.
3. `cargo test --all-features` exits 0, with every test above present and passing.
4. `cargo check --target aarch64-apple-ios` and `--target aarch64-apple-ios-sim` exit 0.
5. A script reads each direct dependency's newest stable version from crates.io and compares it with `Cargo.toml`. Paste the output. Use a generic User-Agent, and no personal details.
6. `grep -rn "println\|dbg!\|eprintln" crates/nsh-secure-storage/src` finds nothing that could print a value.
7. You broke the code on purpose for the per-key binding, the write-during-read rule, and one Apple status mapping, saw each test fail, and restored it. Say how.
8. A real iOS check: in a simulator you created, a small test app built with `nsh test ... --target ios`, or a demo, writes, reads, deletes and lists through the real Keychain. Say what you ran. If it cannot be done, say exactly why.
