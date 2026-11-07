# Brief: TypeScript package, host modules and docs

Read `docs/brief-common.md`, `docs/decisions.md`, `crates/nsh-secure-storage/src/native.rs` or wherever `#[native_module]` lives, and `~/fo/nsh-starter/.agents/skills/nsh/references/mixed.md` and `nsh-host-embedding/SKILL.md` first, then upstream's `lib/flutter_secure_storage.dart` and R10, R14 to R16, R23, R28, R31 in `docs/requirements.md`. You own `packages/nsh-secure-storage/**`, `native/ios/**`, `native/android/SecureStorageHost.kt`, `examples/**`, `README.md`, and `docs/platforms.md`. Do not edit `crates/` or `native/android/` Java and build files, which another agent owns. If the native class lacks something, say so in your report.

## The package

TSX with TanStack-style care for NSH's rules. `import { SecureStorage, useSecureValue } from "nsh-secure-storage"`, with upstream's names in camelCase:

- `new SecureStorage({ iOptions, mOptions, aOptions, lOptions, wOptions, webOptions })`
- `write({ key, value, ... })`, `read`, `containsKey`, `delete`, `readAll`, `deleteAll`, `registerListener`, `unregisterListener`, `unregisterAllListenersForKey`, `unregisterAllListeners`, `isCupertinoProtectedDataAvailable`, `onCupertinoProtectedDataAvailabilityChanged`
- `useSecureValue(key, options?)`: a reactive read with `value()`, `state()`, `error()`, `refresh()`. A binding on one key wakes only for that key, proved by a run counter. A write made during a slow read wins. The subscription ends through `cleanup`.
- Every Promise rejects with an `Error` that has a `code`, and the message never holds a value.

**Three backends**, chosen by the caller through a small explicit import so the bundler never fails on a missing crate:

1. `nsh-secure-storage/native`: the Rust crate's `#[native_module]`, for apps that list the crate in `nsh.json` under `"rust"`. Find how the bundler handles this specifier, and make the type declarations work.
2. `nsh-secure-storage/host`: for signed TSX packages with no Rust crate, over `host.call("secureStorage.*")`, which needs the `host` capability. A refused capability gives `ERR_ACCESS_DENIED` through as a coded error. `native/ios/SecureStorageHost.swift` and `native/android/SecureStorageHost.kt` implement the channel for embedding hosts, with Keychain and AndroidKeyStore. Keep them small and use only platform APIs. They must follow the same rules: no reset on a transient error, and no plaintext on disk. Compile the Swift with `swiftc -typecheck` against the iOS SDK. Kotlin: compile if the toolchain here allows it, and otherwise say so.
3. `nsh-secure-storage/memory`: `SecureStorage.memory(initialValues?)`, upstream's `setMockInitialValues`, for tests.

Write no crypto in TypeScript.

## Tests

In `packages/nsh-secure-storage/tests/`, through a real UI on a manual clock, on the memory backend: every method, listeners, the reactive read with run counters, a slow read losing to a write, cleanup, and error codes. A screen in the root `App.tsx`. Then one real run through the Rust crate on macOS: `nsh test --target macos` with a throwaway keychain file path option, with no dialog, no login keychain, and a check for a new `SecurityAgent` process. If that cannot hold, say so and do not run it.

## Docs

- `README.md`, replacing the placeholder: what this is, install for Rust, Rust with TSX and signed TSX, a sample for each that you have compiled, a table of what backs each platform, the crates it reuses and why, and a short section on what it learned from upstream's bug reports, linking `docs/upstream-triage.md`.
- `docs/platforms.md` (R23): for each platform, what is protected and from whom, what survives uninstall, reinstall, backup and device transfer, size limits, and what was tested here on which device and what was not. Be plain. Web has no real secret store. Windows has not been run.
- `examples/demo-rs` and `examples/demo-ts`, small and working.

## Finish conditions

1. In `packages/nsh-secure-storage`: `nsh check` exits 0, `nsh build` has no `warn` line, and `nsh test` exits 0.
2. A consumer project in a new folder under `/private/tmp/claude-501/-Users-hmziq--claude-spares-spare-NGupus/5378586f-1e26-4b17-89e9-b3eb305cfd38/scratchpad/verify/work/` with the package linked into `node_modules` and the crate in `"rust"`: `nsh check`, `nsh build` and `nsh test` pass.
3. Every sample in the README compiles.
4. A run-counter test proves one key's binding does not wake for another key.
5. You broke the code on purpose once per group (reactive per-key, error codes, host backend), saw the test fail and restored it.
