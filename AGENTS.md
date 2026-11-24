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
- Compile and test on GitHub Actions, not on a developer's machine. Push, then read the results with `gh run`.
- The main branch is `master`.

## Releasing

Add a `## [X.Y.Z] - YYYY-MM-DD` entry at the top of `CHANGELOG.md` and push to `master`. The Changelog Release workflow bumps the versions, tags `vX.Y.Z`, creates the GitHub release with that entry as its notes and publishes to crates.io. Do not bump versions or push tags by hand. A crates.io version cannot be replaced, so CI must be green before the entry lands. To finish a release that stopped halfway: `gh workflow run changelog-release.yml`, or `gh workflow run publish.yml -f tag=vX.Y.Z` for an older tag.
