# Testing the Windows backend

The Windows backend has been built and linted for `x86_64-pc-windows-gnu` on a Mac, but the real
Credential Manager has not been run. Run these on a Windows machine with Rust (the MSVC toolchain)
and the repository checked out.

```powershell
cd nsh-secure-storage
cargo fmt --all -- --check
cargo clippy -p nsh-secure-storage --all-targets --all-features -- -D warnings
cargo test -p nsh-secure-storage --all-features --test windows
```

`--test windows` runs against keyring-core's mock store and a folder under `%TEMP%`. It never opens
Credential Manager, so it is safe on a machine in daily use.

Checking the real Credential Manager is still to do. It needs a test that writes to a unique
`account_name` and removes its key and values file afterwards, and it must never run against an
account name a person uses.
