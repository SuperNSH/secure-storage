# AGENTS.md

Hard rules for humans and coding agents in this repo.

- This library stores secrets. Never log a value, never put one in an error, and never write one to disk unencrypted.
- Comments are 1-2 lines and state a non-obvious constraint. Never narrate the next line.
- Docs read like a competent human wrote them: direct, concrete, no marketing filler.
- Dead code is deleted. No speculative abstractions.
- Rust gates: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features`.
- TypeScript gates, from `packages/nsh-secure-storage`: `nsh check`, `nsh build`, `nsh test`.
- Keep the Rust and TypeScript versions' semantics the same. A behavior change lands in both, with a test in both.
- Tests never touch the user's own keychain, keyring or credentials. They use the memory backend, a throwaway keychain, or a device made for testing.
- The main branch is `master`.
