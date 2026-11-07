# nsh-secure-storage

Secure key-value storage for [Native Shell](https://nsh.dev) apps, in Rust and TypeScript: one API over each platform's own secret store. It is the Native Shell version of [`flutter_secure_storage`](https://github.com/juliansteenbakker/flutter_secure_storage) 11.2.0, and it fixes the problems reported against that package.

Values are kept in the Keychain on Apple platforms, and in the other stores listed under [Platforms](#platforms). The web has no secret store, so nothing stored there is secret from a script on the page. [`docs/platforms.md`](docs/platforms.md) says what each platform protects, and what survives uninstall, backup and device transfer.

## Install

The crate is not published yet. Use a checkout of this repository, and point at its `crates/nsh-secure-storage` directory.

### Rust

```toml
# Cargo.toml
[dependencies]
nsh-secure-storage = { path = "../secure-storage/crates/nsh-secure-storage" }
```

```rust
// src/lib.rs
use nsh_secure_storage::{SecureStorage, SecureStorageError, SecureStorageOptions};

/// Stores `token` in the platform's secret store.
pub async fn save_token(token: &str) -> Result<(), SecureStorageError> {
    let store = SecureStorage::new(SecureStorageOptions::default());
    store.write("token", Some(token), &Default::default()).await
}

/// Reads the token back; `None` when nothing is stored.
pub async fn load_token() -> Result<Option<String>, SecureStorageError> {
    let store = SecureStorage::new(SecureStorageOptions::default());
    Ok(store.read("token", &Default::default()).await?.map(|v| v.to_string()))
}
```

Every operation takes the store by value and is `async`. A `None` value deletes the key.

### Rust with TSX

An app that has the crate under `"rust"` in `nsh.json` can use the store from TSX. The TypeScript side is the `nsh-secure-storage` package in `packages/nsh-secure-storage`, linked into `node_modules`.

```json
{
  "name": "consumer-ts",
  "id": "dev.example.consumer-ts",
  "version": "0.1.0",
  "build": 1,
  "rust": "../secure-storage/crates/nsh-secure-storage"
}
```

```tsx
// App.tsx
import { mount, Text, View } from "nsh";
import { SecureStorage, useSecureValue } from "nsh-secure-storage";
import { rustBackend } from "nsh-secure-storage/rust";

const storage = new SecureStorage({ backend: rustBackend, iOptions: { accessibility: "firstUnlockThisDevice" } });

function App() {
  const token = useSecureValue("token", { storage });
  return (
    <View class="flex-1 p-4">
      <Text>{`token ${token.state()}`}</Text>
    </View>
  );
}

mount(App);
```

`nsh-secure-storage/native` is the crate's own module, which `nsh-secure-storage/rust` wraps. Import `nsh-secure-storage/rust` only from an app that lists the crate: otherwise the bundle fails, which is the point of keeping it a separate import.

### Signed TSX

A signed package has no Rust crate. It reaches the store of the app that embeds it, through the host's `secureStorage.*` handlers, which need the `host` capability:

```json
{
  "name": "consumer-signed",
  "id": "dev.example.consumer-signed",
  "version": "0.1.0",
  "build": 1,
  "capabilities": { "host": ["secureStorage.*"] }
}
```

```tsx
// App.tsx
import { host, mount, Text, View } from "nsh";
import { SecureStorage, useSecureValue } from "nsh-secure-storage";
import { hostBackend } from "nsh-secure-storage/host";

const storage = new SecureStorage({ backend: hostBackend, iOptions: { accountName: "consumer-signed" } });

function App() {
  const remembered = useSecureValue("remembered", { storage });
  return (
    <View class="flex-1 p-4">
      <Text>{host.available() ? "embedded" : "standalone"}</Text>
      <Text>{`remembered ${remembered.state()}`}</Text>
    </View>
  );
}

mount(App);
```

The embedding app registers the handlers. On iOS, with `NativeShell`:

```swift
import NativeShell

@MainActor
func registerSecureStorage(_ app: NshAppView) {
    for name in SecureStorageHost.names {
        app.handle(name) { data in try SecureStorageHost.handle(name, data: data) }
    }
}
```

On Android, with `NshView`:

```kotlin
import com.longbridge.nsh.NshView
import dev.nsh.securestorage.SecureStorageHost
import dev.nsh.securestorage.SecureStorageHostException

fun registerSecureStorage(nsh: NshView, host: SecureStorageHost) {
    for (name in SecureStorageHost.NAMES) {
        nsh.handle(name) { data, reply ->
            try {
                reply.resolve(host.handle(name, data))
            } catch (error: SecureStorageHostException) {
                reply.reject(error.message)
            }
        }
    }
}
```

The two host files are `native/ios/SecureStorageHost.swift` and `native/android/SecureStorageHost.kt`. The Swift sample typechecks against `NshAppView`'s `handle` signatures copied from NativeShell 0.4.0. The Kotlin sample compiles against NSH's own `NshView` sources at 0.4.0. Neither has run inside a host app yet.

## TypeScript API

```ts
new SecureStorage({ backend, iOptions?, mOptions?, aOptions?, lOptions?, wOptions?, webOptions? })
SecureStorage.memory(initialValues?)
```

| Method | Result |
|---|---|
| `write({ key, value })` | Stores `value`. A `null` value deletes the key. |
| `read({ key })` | The value, or `null`. |
| `containsKey({ key })` | Whether a value is stored. |
| `delete({ key })` | Removes the key. An absent key is not an error. |
| `readAll()` | Every entry, in key order, as `{ key, value }`. |
| `deleteAll()` | Removes every key. Listeners of removed keys hear `null`. |
| `registerListener({ key, listener })` | Calls `listener` after each write or delete of `key` through this handle. Returns the stop function. |
| `unregisterListener`, `unregisterAllListenersForKey`, `unregisterAllListeners` | Ends listeners. |
| `listenerCount(key?)` | How many listeners are registered. |
| `isCupertinoProtectedDataAvailable()` | Whether iOS protected data is readable now. |
| `onCupertinoProtectedDataAvailabilityChanged(listener)` | Calls `listener` when that changes. Returns the stop function. |
| `useSecureValue(key, { storage })` | A reactive read: `value()`, `state()`, `error()`, `refresh()`. |

Every method rejects with a `SecureStorageError` whose `code` is one of the `ERR_SECURE_STORAGE_*` codes below. The message names the key, never a value.

| Code | Meaning |
|---|---|
| `ERR_SECURE_STORAGE_LOCKED` | The store is locked. Retry after unlock. |
| `ERR_SECURE_STORAGE_NOT_AVAILABLE` | The platform cannot do this here. |
| `ERR_SECURE_STORAGE_KEY_INVALIDATED` | The key for the data was invalidated. |
| `ERR_SECURE_STORAGE_CORRUPT` | The stored data cannot be decoded or failed its integrity check. |
| `ERR_SECURE_STORAGE_AUTH_CANCELLED`, `ERR_SECURE_STORAGE_AUTH_FAILED` | The user cancelled, or authentication failed. |
| `ERR_SECURE_STORAGE_MISSING_ENTITLEMENT` | The app is not signed with the keychain access group it asks for. |
| `ERR_SECURE_STORAGE_INVALID_KEY` | The key is empty or contains a NUL character. |
| `ERR_SECURE_STORAGE_BACKEND` | The platform's store failed. |

A refused host capability passes through as `ERR_ACCESS_DENIED`.

### Reactive reads

`useSecureValue` follows the writes made through the same `SecureStorage` handle. A binding that reads one key runs again only when that key changes. A read that was overtaken by a write or a `refresh()` drops its answer, so a slow read never overwrites a newer write.

## Backends

Each backend is chosen by an explicit import, so a bundle never pulls in a platform it does not use.

| Import | Backend | Use it for |
|---|---|---|
| `nsh-secure-storage/rust` | The crate's store (`nsh-secure-storage/native`) | Apps that list the crate under `"rust"` |
| `nsh-secure-storage/host` | The embedding app's `secureStorage.*` handlers | Signed TSX packages with no crate |
| `nsh-secure-storage/memory` | Memory, with `memoryBackend({ initialValues, readDelayMs })` | Tests, and data that should not persist |
| `nsh-secure-storage` | `SecureStorage.memory()` | The same memory store, with no extra import |

The host protocol, for anyone who writes a host in another language. Each call carries `options: { ios, macos, android }`, the store's platform options.

| Handler | Data | Reply |
|---|---|---|
| `secureStorage.read` | `{ key }` | `{ value }`, `value` `null` when absent |
| `secureStorage.write` | `{ key, value }` | nothing |
| `secureStorage.delete` | `{ key }` | nothing |
| `secureStorage.containsKey` | `{ key }` | `{ present }` |
| `secureStorage.readAll` | nothing | `{ entries: [{ key, value }] }` |
| `secureStorage.deleteAll` | nothing | nothing |
| `secureStorage.protectedDataAvailable` | nothing | `{ available }` |
| event `secureStorage.protectedDataChanged` | `{ available }` | sent by the host |

A host rejects with a message that starts with the code, as `ERR_SECURE_STORAGE_LOCKED: text`. The TypeScript side keeps the code and drops the text.

## Platforms

| Platform | Store | State |
|---|---|---|
| macOS | Keychain, through the Apple backend | Built. The crate's Apple tests and a 1 MiB round trip on a file keychain pass. Saving through the TSX runner fails on a throwaway file keychain; the cause is open (see `docs/platforms.md`). |
| iOS | Keychain, through the Apple backend | Builds for the iOS simulator (`cargo check`). Not run on a device or simulator here. |
| iOS embedding | `native/ios/SecureStorageHost.swift` | Typechecked against the iOS SDK. Not run in a host app. |
| Android embedding | `native/android/SecureStorageHost.kt` | Compiled with the Kotlin 2 compiler against `android-37.0`. Not run on a device. |
| Android | AndroidKeyStore, through the crate | Pending. |
| Linux | Secret Service, through the crate | Pending. |
| Windows | Credential Manager, through the crate | Pending. Not run: there is no Windows machine here. |
| Web | WebCrypto, through the crate | Pending. |

See [`docs/platforms.md`](docs/platforms.md) for what each row protects and what survives uninstall, backup and transfer.

### Crates reused, and why

The crate builds on `security-framework` for the Keychain. Apple's keyring crate uses the same library, but it cannot set the per-item label, comment, description or the prompt text that upstream's options need, so the Apple backend is ours. `zeroize` keeps values out of memory after use. Windows and Linux are planned on `keyring-core`, with the store behind the same `Backend` trait. Android uses its own JNI backend, because `android-native-keyring-store` keeps data in SharedPreferences, which Auto Backup includes. The decisions are recorded in [`docs/decisions.md`](docs/decisions.md).

## What we learned from upstream

Upstream's issues and pull requests were all read and sorted into clusters: [`docs/upstream-triage.md`](docs/upstream-triage.md). The points that shaped this library:

- Writes wait for the earlier ones, and a read sees the latest finished write. A slow read never overwrites a newer write.
- A failed lookup never replaces or deletes a key. Only an explicit, permanent signal may lead to data loss, and only when the caller opts in with `resetOnError`. It defaults to `false`, unlike upstream, because silently losing a user's data is worse than reporting an error.
- A value never appears in an error, a log or a `Debug` output.
- Missing keys read as `null` and deleting an absent key succeeds. An empty string is a value, not an absence.
- Errors carry a stable `code`, so apps can match them.

## Developing

```sh
cd packages/nsh-secure-storage
nsh check
nsh build
nsh test
```

The macOS keychain tests use a throwaway file keychain in a temp folder and turn keychain interaction off for the whole test process. They never open the login keychain, and no test opens a system dialog.
