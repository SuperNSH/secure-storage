# Brief: real iOS run and the core's open requirements

Read `docs/brief-common.md` and `docs/decisions.md` first. You own `crates/nsh-secure-storage/src/{store,reactive,backend,error,options,memory}.rs`, `src/apple/**`, `tests/{core,reactive,apple,docs}.rs` and `docs/decisions.md`. Others are adding `src/android/`, `src/windows/`, `src/linux/`, `src/web/`, and lines in `src/lib.rs` and `Cargo.toml`, so keep edits there small.

## 1. A real iOS run

The earlier agent could not finish this. NSH 0.4.0's `nsh-platform-ios` on crates.io has `#[path = "../../nsh-platform-macos/src/speech.rs"]`, and that file is not in the published crate, so any Rust build for iOS fails with `couldn't find file`. A `[patch]` in the app's `Cargo.toml` did not reach the development host, which `nsh` generates itself.

Use a `.cargo/config.toml` in the demo app's folder, with `[patch.crates-io]` pointing NSH's crates at `~/fo/.refs/nsh-v0.4.0/crates/*`. Patch every NSH crate that has a path dependency on another, so one source is used. Do not change `~/.cargo`, the registry, or NSH itself. The demo is at `/private/tmp/claude-501/-Users-hmziq--claude-spares-spare-NGupus/5378586f-1e26-4b17-89e9-b3eb305cfd38/scratchpad/secure-demo`, and your own simulator is `nsh-secure-ios`, UDID `98548CCF-22B6-414B-8F79-96D3AE57638C`, already booted. Do not touch the simulator named `nsh-auth`. Also try `nsh package App.tsx --target ios-simulator --run` if `nsh test --target ios` will not run.

Write, read, list and delete through the real Keychain, through a UI. Then check Face ID on the simulator: enroll with `xcrun simctl spawn <udid> notifyutil -s com.apple.BiometricKit.enrollmentChanged 1` and post it with `notifyutil -p`, and match with `com.apple.BiometricKit_Sim.fingerTouch.match` or the Face ID equivalent. An item with `biometryAny` must prompt, pass on a match, and give `ERR_SECURE_STORAGE_AUTH_FAILED` or `_AUTH_CANCELLED` on no match. If simulator Face ID cannot be driven from a command, say exactly what you tried.

Record the workaround in `docs/decisions.md`, and say plainly that it is an upstream NSH packaging bug.

## 2. Close the core's open requirements

Read each in `docs/requirements.md`:

- R3: move `reset_on_error`'s rule into the core so a backend cannot recurse: at most one retry after a reset, and never on a transient error. Test `reset_on_error_runs_at_most_once` with a backend that fails on the retry too.
- R9: there is no "init" step here, so test the real risk: two handles and two surfaces run their first operations at once, in order.
- R10: test a backend that panics, through the real call path: the caller gets a coded error, and the store stays usable.
- R12: a write to an existing Apple item must be an update. The test must be able to see a delete-then-add: compare the item's creation date before and after, on a file keychain.
- R26: `backend_calls_run_off_engine_thread`: a backend records its thread id, and the test compares it with the engine thread's.
- R2: test on Apple too: a transient failure does not delete the item.

`AuthenticationUi::Fail` already has a counted guard. Check two concurrent calls cannot leave interaction off.

## Finish conditions

1. `cargo fmt --all -- --check`, `cargo clippy -p nsh-secure-storage --all-targets --all-features -- -D warnings`, and `timeout 300 cargo test -p nsh-secure-storage --all-features` exit 0, with no new `SecurityAgent` process.
2. Paste the iOS run's output, showing real Keychain results, and a screenshot path from `xcrun simctl io <udid> screenshot`.
3. Every requirement above has its test, and you broke the code once per requirement group and saw the test fail.
