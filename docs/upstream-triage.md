# Upstream triage: flutter_secure_storage issues and pull requests

Source: `juliansteenbakker/flutter_secure_storage`, read with `gh` on 2026-11-07. 91 clusters cover 1299 items: 721 issues and 578 pull requests. Every open issue and open pull request has a row, a cluster and a status that is not `no action`.

## Summary

Counts per status:

| Status | Items |
|---|---:|
| requirement | 414 |
| avoided by design | 152 |
| not applicable | 597 |
| no action | 136 |
| total | 1299 |

Counts per state:

| Type | State | Items |
|---|---|---:|
| PR | CLOSED | 218 |
| PR | MERGED | 348 |
| PR | OPEN | 12 |
| issue | CLOSED | 711 |
| issue | OPEN | 10 |

Method: every item was classified from its title, and each cluster names its root cause. All 10 open issues and all 12 open pull requests were read. Open issue bodies were read to their first 4,000 characters, which hold the report and its stated cause; the rest is log and doctor output. Open PR bodies were read in sections, and every open PR's changed-file list was read. Comments were read with `gh issue view --comments` for each open issue that has comments; only the tail of each thread was read, which holds the latest and most substantive comment. About 75 closed failure reports were read in their body, and the cluster row names the reports behind each cluster. Other closed items were classified by title alone.

## Clusters

| Cluster | What goes wrong | Root cause | Items | Status | Requirement or reason |
|---|---|---|---:|---|---|
| Web: first writes on a fresh store each create a key (WEB-KEY) | Concurrent first writes leave values that cannot be decrypted again. | The key check and the key store are separated by awaits, so the last store wins. | 2 | requirement | R1 |
| Web: an undecryptable entry throws from read or readAll (WEB-READ) | read throws a DOMException instead of returning null. | A JS DOMException is not a Dart Exception, so the catch clause misses it. | 6 | requirement | R29 |
| Web: WebCrypto unavailable outside a secure context (WEB-CONTEXT) | crypto.subtle is undefined on http, IP hosts and some private modes; the browser evicts site storage. | WebCrypto needs a secure context, and browsers can clear site storage. | 13 | requirement | R30 |
| Web: deleteAll or delete removes too much, or fails (WEB-DELETE) | Clearing one store wipes all of localStorage, or delete has no effect. | Web deletes are not scoped to the store's own keys. | 4 | requirement | R11 |
| Web: js, dart:html, WASM and package upgrades (WEB-PKG) | Build conflicts from old js or dart:html imports. | Dart package wiring of the Flutter web implementation. | 29 | not applicable | Dart package wiring; the NSH web backend is Rust and WebCrypto with no js package. |
| Web: support questions and general requests (WEB-ASK) | Users ask whether web is supported or how keys are created. | Questions answered by the design and the platform section of the docs. | 16 | no action | Question or request; answered by R23 and R30. |
| Linux: a dropped libsecret store is reported as success (LINUX-VERIFY) | write returns normally, read returns null, or one dropped replace erases other keys. | libsecret replaces an item by delete then create, and the plugin keeps all keys in one item. | 3 | requirement | R7 |
| Linux: locked, missing, cold or sandboxed keyring (LINUX-KEYRING) | read returns null or hangs, or the schema attribute goes stale after relabelling. | libsecret errors are dropped or mapped to null; the schema name dangles after setLabel. | 21 | requirement | R8 |
| Linux: CMake, libsecret headers, linker and snap builds (LINUX-BUILD) | The plugin does not build on a distribution or inside a snap. | Build-system wiring of the Flutter Linux plugin. | 29 | not applicable | CMake and distro packaging; the NSH Linux backend is built by Cargo. |
| Linux: where values live and which keyring is needed (LINUX-DOC) | Users do not know the keyring requirement or where values are stored. | Documentation gap. | 7 | requirement | R23 |
| Native calls block the UI or the engine thread (BLOCK) | The UI hangs while a key is created or a keyring call waits. | Blocking backend work runs on the caller's thread. | 4 | requirement | R26 |
| A missing key or missing folder is reported as an error (MISSING) | read or delete of an absent key throws or crashes. | The not-found result is not separated from failure. | 7 | requirement | R15 |
| containsKey answers differently from read (CONTAINS) | containsKey is always true or always false on some platforms, or is missing. | containsKey is implemented from the read path, and not on every backend. | 14 | requirement | R14 |
| Empty value and null semantics (EMPTY) | Writing null or an empty string deletes or is mistaken for a missing key. | Null is used as the delete signal. | 1 | requirement | R16 |
| Reserved characters in key names (KEY-CHARS) | A key with a colon or a slash crashes the app or creates an unreadable file. | Key names are used as file names without encoding. | 1 | requirement | R17 |
| Large values and slow writes (LARGE) | Writes above a few kilobytes fail or take seconds. | No documented size limit and no test at 1 MiB on each backend. | 3 | requirement | R18 |
| Concurrent reads and writes return stale, null or missing values (ORDER) | A read during or just after a write returns null, an old value, or skips keys. | Operations on one store are not ordered by a revision. | 11 | requirement | R28 |
| Errors and logs carry noise or values (LOG) | Logs show repeated error lines or stack traces with values. | Error text is built from the platform message, which can hold data. | 2 | requirement | R31 |
| Crashes without a clear backend cause (CRASH-MISC) | The app aborts with a native crash or EXC\_BAD\_ACCESS. | Not identifiable from the report; the requirement is a coded error instead of an abort. | 10 | requirement | R10 |
| Docs: what each platform protects, and what survives uninstall or backup (DOCS) | Users assume secrets are safe in every store, including the web. | Documentation gap about platform stores. | 13 | requirement | R23 |
| Feature requests outside the design (FEATURE) | Requests for typed values, streams, isolates, extra file stores or hardware modes. | Scope of the library. | 17 | no action | Feature request outside the design; not a defect. |
| Android: a Keystore lookup failure is read as a missing key (AND-READ) | A transient null from getKey or getCertificate makes the plugin generate a key or delete data. | Keystore binder failures return null, and the plugin treats null as absence. | 28 | requirement | R2 |
| Android: a missing platform class or backend error crashes the app (AND-LOAD) | NoClassDefFoundError, NoSuchFieldError or an NPE escapes catch(Exception) and aborts the process. | Errors other than Exception are not caught, and optional platform classes load eagerly. | 14 | requirement | R10 |
| Android: reset\_on\_error recurses or resets too eagerly (AND-RESET) | The reset path calls itself until the process dies; a locked keystore counts as a key mismatch. | No attempt guard on the reset path; UserNotAuthenticated is treated as InvalidKey. | 9 | requirement | R3, R4 |
| Android: biometric key invalidated, or per-operation prompts lost (AND-BIO-KEY) | A key invalidated by a fingerprint change cannot be recovered; the per-operation prompt stops after a migration. | Invalidation is not routed to recovery; the cipher is cached after a migration. | 10 | requirement | R5, R6 |
| Android: biometric prompt hangs, crashes or replies twice (AND-BIO-PROMPT) | Cancel or the negative button never resolves the call; callbacks run twice. | Callbacks are not resolved exactly once on every path. | 11 | requirement | R27 |
| Android: two engines or handles share one store (AND-SERIAL) | A second engine creates a second RSA pair under the same alias, and the store becomes unreadable. | Initialization is not serialized across engines or handles in one process. | 3 | requirement | R9 |
| Android: store names collide and deleteAll is too broad (AND-STORE-ID) | deleteAll clears other stores, and options under one name are ignored or shared. | The store name is not part of the file and key prefix, and deleteAll clears the whole file. | 13 | requirement | R11 |
| Data lost without a clear cause (Android and iOS) (AND-LOSS) | Values disappear after hours, restarts or updates; the reports name no cause. | Not determinable from the reports; the requirement is no silent deletion and coded errors. | 15 | requirement | R2, R21 |
| Android: Auto Backup, reinstall and device transfer (AND-BACKUP) | Ciphertext returns without its Keystore key, reads fail, or values vanish after a reinstall. | Keystore keys are not backed up, while the values file can be restored from backup. | 11 | requirement | R22 |
| Android: Flutter embedding, Activity lifecycle and channels (AND-EMB) | Plugin registration or Activity pausing breaks, or channel names differ. | Flutter embedding lifecycle; the NSH Android backend has no Flutter channel. | 10 | not applicable | Flutter embedding lifecycle; the NSH Android backend has no Flutter channel. |
| Android: Gradle, AGP, Kotlin, minSdk and build wiring (AND-PKG) | The plugin does not build or conflicts with the host Gradle setup. | Gradle and AGP wiring of the Flutter Android plugin. | 45 | not applicable | Gradle and AGP wiring; the NSH Android backend is built by Cargo and the NDK. |
| Android: legacy ciphers, padding and key formats (CRYPTO-LEGACY) | AES-CBC or ECB with PKCS padding, RSA wrapping, and old key formats. | Cryptography of older plugin versions. | 76 | avoided by design | Our Android design uses one AES-256-GCM key in AndroidKeyStore, with no CBC, ECB, PKCS padding or RSA wrap. |
| Android: EncryptedSharedPreferences and Tink keysets (ESP-TINK) | Migration into or out of EncryptedSharedPreferences fails or loses data. | Jetpack Security and Tink belong to older releases. | 10 | avoided by design | Our design has no EncryptedSharedPreferences and no Tink, so no ESP or Tink migration exists. |
| Android: plaintext fallback when encryption fails (AND-FALLBACK) | A preferences fallback stores values unencrypted. | Fallback to a plain store when the cipher fails. | 1 | avoided by design | Rule 2 in docs/brief-common.md: a value is never written unencrypted, so no fallback exists. |
| Option names and types drift between Dart and native code (AUTH-NAMES) | An option name or type differs between layers and is silently ignored. | Dart-to-native option mapping. | 4 | avoided by design | Our options are typed Rust structs, so names cannot drift between layers. |
| Windows: Credential Manager size limit (WIN-SIZE) | Secrets over about 2.5 KB fail with UNKNOWN\_ERROR. | The Credential Manager blob limit applies to the value itself. | 3 | avoided by design | Our design keeps values in an AES-GCM file and only the AES key in Credential Manager. |
| Windows: values shared between apps (WIN-SHARE) | Two apps with the same key name read each other's values. | The Credential Manager target name is not scoped to the app. | 7 | requirement | R19 |
| Windows: values file corrupted or locked by another process (WIN-FILE) | Users lose the store at random; PathAccessException on restart; writes truncate. | The values file is not written atomically and a locked file is not retried. | 7 | requirement | R20 |
| Apple: items not found after an accessibility, group or app change (APP-QUERY) | read returns null after the accessibility level, access group or hot restart changed. | Accessibility is used as a search filter, and the query differs between read and write. | 31 | requirement | R13 |
| Apple: write fails on an existing item, or leaves duplicates (APP-UPSERT) | -25299 already exists, or a write removes the old item before adding the new one. | Write is delete then add, not one atomic update. | 10 | requirement | R12 |
| Apple: OSStatus codes and missing entitlements (APP-STATUS) | A missing item is an error, or a missing entitlement crashes or looks like data loss. | Status codes are not mapped; entitlement and provisioning failures are not reported. | 9 | requirement | R25 |
| Apple: locked device or background access (APP-LOCKED) | read returns null or hangs while the device is locked or the app is in the background. | errSecInteractionNotAllowed is not surfaced as a coded error. | 7 | requirement | R24 |
| Apple: accessibility, access control and option names (APP-OPTS) | Accessibility options are not validated or do not map to the Keychain constants. | Options are not one-to-one with kSecAttrAccessible, and there is no ThisDeviceOnly default. | 14 | requirement | R21 |
| Apple: items survive uninstall and restore from iCloud backup (APP-BACKUP) | A reinstalled app reads the secrets of an earlier install, or users want backups off. | Keychain items are not removed on uninstall, and they restore from iCloud unless ThisDeviceOnly. | 18 | requirement | R21 |
| Apple: iCloud Keychain sync (kSecAttrSynchronizable) (APP-SYNC) | The synchronizable flag is dropped or applied to some queries only. | Sync is not part of our design, and no option exposes it. | 7 | avoided by design | Our design has no iCloud Keychain sync option, so no query can carry or drop the flag. |
| Apple: Swift Package Manager, Xcode, CocoaPods and macOS packaging (APP-PKG) | The plugin fails to build, or is not offered through Swift Package Manager. | Flutter packaging of the Apple plugin. | 44 | not applicable | Flutter Apple packaging; the NSH Apple backend is a Rust crate that uses security-framework. |
| Windows: C++, win32, build and FFI wiring (WIN-PKG) | The plugin fails to compile or is pinned to an old win32 version. | Flutter Windows plugin packaging and win32 dependency. | 21 | not applicable | Flutter Windows packaging; the NSH Windows backend is a Rust crate on DPAPI and Credential Manager. |
| Flutter: MissingPluginException, Dart null checks and mocking (FLUTTER-WIRE) | A channel is not registered, a Dart null check fails, or a mock is wrong. | Dart and Flutter wiring; none of it exists in the Rust crate. | 55 | not applicable | Dart and Flutter wiring; not part of the Rust crate. |
| Dependency bumps, release PRs and CI (DEPS) | Version bumps, release-please PRs and CI changes. | Repository maintenance. | 295 | not applicable | Repository maintenance; NSH dependency policy is in docs/brief-common.md. |
| Android below API 23 or Android 4.4 to 5.x (ANDROID-MINAPI) | Crashes or missing features on old Android releases. | Old platform APIs. | 6 | avoided by design | Our Android backend needs API 23 for an AES-GCM AndroidKeyStore key, so older releases are out of scope. |
| Android: RSA key wrapping and locale-dependent key generation (ANDROID-RSA) | RSA key generation fails for some locales or old API levels. | RSA wrapping of the AES key, which our design does not use. | 4 | avoided by design | Our design wraps nothing with RSA; the AES key lives in AndroidKeyStore. |
| Android: initialization log loop (ANDROID-INIT-LOOP) | Logs repeat 'ensureInitStorageCipher' and 'already initialized' lines. | Legacy initialization path with several entry points. | 3 | avoided by design | Our Android backend has one initialization path per store, so this loop cannot occur. |
| Finalizers on Keystore objects (NO-FINALIZER) | Exceptions from finalizers of KeyStore objects. | Java finalizers. | 1 | avoided by design | The Rust backend has no finalizers; drop is deterministic. |
| Java and C++ memory management and locking (NATIVE-MEMORY) | Memory leaks, double-checked locking, and free-after-use in native code. | Manual memory and locking in the Java and C++ plugin code. | 5 | avoided by design | Rust ownership covers memory; the backend takes one Mutex per store (R9) and zeroizes values (rule 1). |
| Upgrades from plugin versions 3 to 10 (LEGACY-VERSION) | Data written by an old version is unreadable or lost after the upgrade. | Key and file formats changed between versions. | 7 | avoided by design | NSH has no legacy data to migrate (docs/brief-triage.md, Method section). |
| Lint, style, deprecation warnings and refactors (NA-LINT) | Analyzer or compiler warnings and style changes. | Code hygiene of the Flutter plugin. | 23 | not applicable | Lint and style of the Flutter plugin; NSH uses clippy and its own checks. |
| Build and tooling errors with no runtime cause (NA-BUILD) | The project does not build, or a tool version is wrong. | Flutter, Gradle, AGP, Xcode and CocoaPods wiring. | 29 | not applicable | Flutter and Gradle wiring; not part of the Rust crate. |
| Android embedding and Activity wiring (extra) (NA-EMB) | Flutter embedding or locale wiring. | Flutter embedding lifecycle. | 4 | not applicable | Flutter embedding lifecycle. |
| Flutter and Dart wiring (extra) (FLUTTER-WIRE-2) | Dart-side types, null safety, and test helpers. | Dart and Flutter, not in the Rust crate. | 5 | not applicable | Dart and Flutter wiring. |
| Android default option names (extra) (AUTH-NAMES-2) | Default cipher names differ between the Dart and Android sides. | Option mapping between layers. | 1 | avoided by design | Options are typed Rust structs; names cannot drift. |
| Questions, duplicates, feature notes and reports without detail (NOA-MISC) | Usage questions, duplicates, status notes, or reports with no reproduction. | Not a defect in the report as written. | 87 | no action | Question, duplicate, status note or incomplete report; no defect to fix. |
| Feature requests outside the design (extra) (FEATURE-2) | Requests for hardware modes, listeners, sessions or new options. | Scope of the library. | 14 | no action | Feature request outside the design; not a defect. |
| Data lost after an upgrade or update (extra) (LOSS-2) | Values disappear after an app update, or the Android store is cleared. | No cause in the report; the requirement is no silent deletion. | 2 | requirement | R2, R21 |
| Crashes and exceptions without a backend cause (extra) (CRASH-2) | NPE, crash on launch or unhandled exception in the plugin. | Not identifiable from the report; the requirement is a coded error. | 14 | requirement | R10 |
| Apple query identity and configuration changes (extra) (IOS-QUERY-2) | Items missing after a configuration change, access group, or native migration. | Query identity differs between writes and reads. | 15 | requirement | R13 |
| Apple: rewrite of an existing value (extra) (UPSERT-2) | A write over an existing value does nothing. | Write is not an update. | 1 | requirement | R12 |
| Android: Keystore failure on first run or on some devices (extra) (KEYSTORE-READ-2) | Keystore exceptions on first start or after a device change. | A Keystore failure is treated as absence or fatal. | 7 | requirement | R2 |
| Android: init ordering and singleton (extra) (INIT-SERIAL-2) | Init runs too late or twice, and several instances are created. | One instance per store is not enforced. | 3 | requirement | R9 |
| Store identity: names, prefixes and web deleteAll (extra) (NAMES-R11-2) | Store name or prefix is ignored, or deleteAll clears too much. | Store identity is not used in keys. | 5 | requirement | R11 |
| containsKey on Android and iOS (extra) (CONTAINS-2) | containsKey is missing or wrong. | Separate code path from read. | 2 | requirement | R14 |
| Linux: keyring errors and cold keyrings (extra) (LINUX-2) | AppArmor denial, cold keyring search, or keyring handling. | libsecret errors are not coded. | 3 | requirement | R8 |
| Android: access control and biometric error codes (extra) (BIO-2) | Biometric access control or error mapping. | Options and error codes for biometrics. | 1 | requirement | R5 |
| Android: biometric error codes (extra) (BIO-CODES-2) | Biometric error codes are not surfaced. | Errors from BiometricPrompt are not coded. | 1 | requirement | R27 |
| Apple: entitlements and protected data (extra) (APP-ENT-2) | Entitlement errors or protected-data availability. | Provisioning and protected-data state are not reported. | 3 | requirement | R25 |
| Apple: protected data availability (extra) (LOCK-2) | The library does not tell the app that protected data is unavailable. | No availability check. | 1 | requirement | R24 |
| Read after write, readAll, and blocking calls (extra) (ORDER-2) | A read returns null after a write, readAll fails, or the UI thread blocks. | Ordering is not enforced, and calls run on the wrong thread. | 6 | requirement | R26, R28 |
| Web: key not found and reads (extra) (WEB-CONTEXT-2) | Web reads fail after a restart. | Web key handling. | 1 | requirement | R30 |
| Android read speed (extra) (PERF-2) | First read is slow because the cipher is created early. | Cipher created before it is needed. | 1 | requirement | R18 |
| Logging noise (extra) (LOG-2) | Debug logs in the native layer. | Logging left on in the Android layer. | 1 | requirement | R31 |
| Apple: exposing options (extra) (OPTS-2) | Keychain options are not exposed. | Options are not typed. | 1 | requirement | R21 |
| Docs: delete, Apple options, and macOS (extra) (DOCS-2) | Docs do not explain delete, Apple options, or Keychain Sharing. | Documentation gap. | 5 | requirement | R23 |
| Android: 16 KB page size (PAGE-16K) | Native libraries must be 16 KB aligned for newer Android releases. | Native library alignment set at build time. | 1 | requirement | R32 |
| Windows: large object error (extra) (WIN-SIZE-2) | A large object fails on Windows. | Credential Manager size limit. | 1 | avoided by design | Our design keeps values in an AES-GCM file and only the AES key in Credential Manager. |
| Android: legacy encryption and wrapping (extra) (CRYPTO-LEGACY-2) | Legacy cipher or padding bugs, RSA wrap, and the rewrite of the cipher code. | Cryptography of older plugin versions. | 13 | avoided by design | Our Android design uses one AES-256-GCM key in AndroidKeyStore. |
| Dependency and release PRs not caught by title rules (LAST-DEPS) | Version bumps and release preparation. | Repository maintenance. | 7 | not applicable | Repository maintenance; NSH dependency policy is in docs/brief-common.md. |
| EncryptedSharedPreferences and Tink, last items (LAST-ESP) | Jetpack Security and Tink updates. | Older plugin releases only. | 5 | avoided by design | Our design has no EncryptedSharedPreferences and no Tink. |
| Last items: docs, lint and Zero-Tap sign-in (LAST-MISC) | Docs, lint, or a Google sign-in restore feature. | Outside the storage library. | 2 | no action | Docs note, lint note or feature outside the library; no defect to fix. |
| Web exception handling (PR) (LAST-WEB) | Web read throws instead of returning null. | Exception handling on web. | 1 | requirement | R29 |
| Deprecation warning (LAST-LINT) | Compiler warning. | Code hygiene. | 1 | not applicable | Lint of the Flutter plugin. |
| Android: hardcoded, custom or exposed encryption keys (KEY-HANDLING) | A key is hardcoded, supplied by the app, or sits in a file readable on a rooted device. | Key material handled outside the Keystore. | 5 | avoided by design | Our AES key is generated inside AndroidKeyStore; no key is hardcoded, supplied by the app, or stored in a file. |

## Items

| # | Type | State | Title | Cluster | Status |
|---:|---|---|---|---|---|
| 1 | issue | CLOSED | Deprecation warning | LAST-LINT | not applicable |
| 2 | issue | CLOSED | Bug on Android 4.4.2 | ANDROID-MINAPI | avoided by design |
| 3 | issue | CLOSED | Getting error try run to iOS using flutter beta | APP-PKG | not applicable |
| 4 | PR | MERGED | Fixed KEY\_ALIAS for Android 4.4.2. | ANDROID-MINAPI | avoided by design |
| 5 | issue | CLOSED | Sporadic finalizer exceptions for underlying KeyStore on Android | NO-FINALIZER | avoided by design |
| 6 | issue | CLOSED | getting a IllegalBlockSizeException | CRYPTO-LEGACY | avoided by design |
| 7 | issue | CLOSED | Invalid LICENSE? | NOA-MISC | no action |
| 8 | PR | MERGED | Migrate android to aes | CRYPTO-LEGACY-2 | avoided by design |
| 9 | issue | CLOSED | Initialize plugin with context rather than with activity | AND-EMB | not applicable |
| 10 | issue | CLOSED | Is it possible to empty the entire storage? | NOA-MISC | no action |
| 11 | issue | CLOSED | Is it possible that data is wiped across installs. | AND-BACKUP | requirement |
| 12 | PR | MERGED | Add deleteAll and readAll | NOA-MISC | no action |
| 13 | issue | CLOSED | 3.1.0: Failed to unwrap key | AND-READ | requirement |
| 14 | PR | MERGED | Update .gitignore to new dart\_tool pub cache | DEPS | not applicable |
| 15 | issue | CLOSED | unchecked or unsafe operations | NA-LINT | not applicable |
| 16 | issue | CLOSED | Getting "Could not find method google() for arguments \[\] on repository container." error | AND-PKG | not applicable |
| 17 | PR | MERGED | Add suppress for unchecked warning. | NA-LINT | not applicable |
| 18 | issue | CLOSED | Exception in android\_job\_scheduler | NOA-MISC | no action |
| 19 | issue | CLOSED | version 3.1.1 | NOA-MISC | no action |
| 20 | issue | CLOSED | intermittent failures to register plugin on Android emulator | FLUTTER-WIRE-2 | not applicable |
| 21 | issue | CLOSED | Missing Plugin Exception: implementation not found for method readAll | FLUTTER-WIRE | not applicable |
| 22 | issue | CLOSED | Allow to customize KEYCHAIN\_SERVICE | NAMES-R11-2 | requirement |
| 23 | PR | MERGED | fix bug: initializing plugin failed by cannot unwrap existing key. | KEYSTORE-READ-2 | requirement |
| 24 | issue | CLOSED | Bad Decrypt on Read after upgrading | CRYPTO-LEGACY-2 | avoided by design |
| 25 | issue | CLOSED | Error on Android 5 | ANDROID-MINAPI | avoided by design |
| 26 | issue | CLOSED | Howto upgrade to \>=3.x.x | LEGACY-VERSION | avoided by design |
| 27 | issue | CLOSED | Saving data between installs | AND-BACKUP | requirement |
| 28 | issue | CLOSED | Storage.read returns null on ios but works on android | IOS-QUERY-2 | requirement |
| 29 | issue | CLOSED | How to test without running the Material app? | FLUTTER-WIRE | not applicable |
| 30 | issue | CLOSED | broken build in latest flutter | FLUTTER-WIRE | not applicable |
| 31 | issue | CLOSED | Feature request: add shared KeyStore/KeyChain possibility | FEATURE-2 | no action |
| 32 | issue | CLOSED | KeyStore exception | AND-READ | requirement |
| 33 | PR | CLOSED | Added method contains | CONTAINS-2 | requirement |
| 34 | issue | CLOSED | Warning on Android 9.0 Pie | NA-LINT | not applicable |
| 35 | issue | CLOSED | Read key value from a different class file does to work | NOA-MISC | no action |
| 36 | PR | MERGED | Access keys correctly | NA-LINT | not applicable |
| 37 | PR | CLOSED | Replace Support with AndroidX | NA-LINT | not applicable |
| 38 | issue | CLOSED | Feature request: Android strongbox suppport | FEATURE | no action |
| 39 | issue | CLOSED | check if key exists | CONTAINS-2 | requirement |
| 40 | PR | MERGED | Use StrongBox on devices that support it | FEATURE | no action |
| 41 | PR | MERGED | Add version check for setIsStrongBoxBacked. | FEATURE-2 | no action |
| 42 | issue | CLOSED | 3.2.0 'no implementation found' | FLUTTER-WIRE | not applicable |
| 43 | issue | CLOSED | FlutterSecureStorage.read broken on Android 9.0 Samsung only | KEYSTORE-READ-2 | requirement |
| 44 | issue | CLOSED | fatal error: 'Flutter/Flutter.h' file not found | APP-PKG | not applicable |
| 45 | PR | CLOSED | add testing mock | FLUTTER-WIRE-2 | not applicable |
| 46 | issue | CLOSED | Keystore exception -- | AND-READ | requirement |
| 47 | PR | MERGED | Fix Android 9 exception. | KEYSTORE-READ-2 | requirement |
| 48 | PR | MERGED | Add const constructor | NA-LINT | not applicable |
| 49 | issue | CLOSED | MissingPluginException (MissingPluginException(No implementation found for method read on channel plugins.it\_n | FLUTTER-WIRE | not applicable |
| 50 | issue | CLOSED | iOS: Allow customizing kSecAttrAccessible | APP-OPTS | requirement |
| 51 | issue | CLOSED | iCloud backup | APP-BACKUP | requirement |
| 52 | issue | CLOSED | App Crashes on Startup (Android 4.4) | ANDROID-MINAPI | avoided by design |
| 53 | issue | CLOSED | App built in release mode does not start | NA-BUILD | not applicable |
| 54 | issue | CLOSED | Secure data wiped | AND-LOSS | requirement |
| 55 | issue | CLOSED | How much data can store in flutter\_secure\_storage ? | LARGE | requirement |
| 56 | PR | MERGED | Added groupId for iOS keychain sharing | IOS-QUERY-2 | requirement |
| 57 | issue | CLOSED | flutter\_secure\_storage seems to not install in ios folder if there were existing pods. | APP-PKG | not applicable |
| 58 | issue | CLOSED | No certificate found under alias error | AND-READ | requirement |
| 59 | issue | CLOSED | What are the key and value | NOA-MISC | no action |
| 60 | issue | CLOSED | ECB Mode is Insecure | CRYPTO-LEGACY | avoided by design |
| 61 | PR | MERGED | Added minimum sdk requirement on AndroidManifest | NA-BUILD | not applicable |
| 62 | issue | CLOSED | Android Devices throw error unless values are written as .toString() | FLUTTER-WIRE-2 | not applicable |
| 63 | issue | CLOSED | Read from keychain in iOS project | NOA-MISC | no action |
| 64 | issue | CLOSED | Feature request: Example to use with SQFLITE file | NOA-MISC | no action |
| 65 | issue | CLOSED | FlutterSecureStorage values in Integration Tests | FLUTTER-WIRE | not applicable |
| 66 | PR | CLOSED | Fix #58; fix No certificate found under alias | KEYSTORE-READ-2 | requirement |
| 67 | issue | CLOSED | how to write an array/list data to storage ? | FEATURE | no action |
| 68 | issue | CLOSED | On uninstall delete keys and values from keychain, possible? | APP-BACKUP | requirement |
| 69 | issue | CLOSED | Android cryptography audit: 6 findings | KEY-HANDLING | avoided by design |
| 70 | issue | CLOSED | IOS - readAll() never returns results. | IOS-QUERY-2 | requirement |
| 71 | issue | CLOSED | MissingPluginException | FLUTTER-WIRE | not applicable |
| 72 | PR | MERGED | Fix Gradle version in gradle-wrapper.properties | DEPS | not applicable |
| 73 | issue | CLOSED | Crash without iOSOptions in v3.3.0 | CRASH-2 | requirement |
| 74 | issue | CLOSED | 3.30 Version Issue | NA-BUILD | not applicable |
| 75 | issue | CLOSED | FlutterSecureStoragePlugin header not found when running xcode product archive | APP-PKG | not applicable |
| 76 | PR | MERGED | Add import for documentation completion. | NA-LINT | not applicable |
| 77 | issue | CLOSED | Erro android studio | NA-BUILD | not applicable |
| 78 | issue | CLOSED | Android - Read value after re-install | AND-BACKUP | requirement |
| 79 | issue | CLOSED | android mainfest file sync error | NA-BUILD | not applicable |
| 80 | PR | MERGED | Remove unnecessary manifest change | NA-BUILD | not applicable |
| 81 | issue | CLOSED | Error App crash on start | CRASH-2 | requirement |
| 82 | issue | CLOSED | data is restored after reinstalling the application in IOS | APP-BACKUP | requirement |
| 83 | issue | CLOSED | MissingPluginException(No implementation found for method write on channel plugins.it\_nomads.com/flutter\_secur | FLUTTER-WIRE | not applicable |
| 84 | issue | CLOSED | reading nonexistent key fails on some devices (Android) | MISSING | requirement |
| 85 | issue | CLOSED | doesn't work on api level 23 | CRASH-2 | requirement |
| 86 | issue | CLOSED | passing the data between diffrent screens | NOA-MISC | no action |
| 87 | issue | CLOSED | Is this library alive? | NOA-MISC | no action |
| 88 | issue | CLOSED | iOS restore app from backup | APP-BACKUP | requirement |
| 89 | issue | CLOSED | optional Synchronous method option needed | FEATURE-2 | no action |
| 90 | issue | CLOSED | Enable caching option needed \[enhancement \] | FEATURE-2 | no action |
| 91 | PR | CLOSED | Add macOS plugin. | APP-PKG | not applicable |
| 92 | PR | CLOSED | Feature/lint fix | DEPS | not applicable |
| 93 | PR | MERGED | Feature/lint fix | DEPS | not applicable |
| 94 | issue | CLOSED | \[iOS\] Does not work when keychain accessed when app is in background | APP-LOCKED | requirement |
| 95 | PR | MERGED | Remove the entry when call 'write' with a null for 'value' param | EMPTY | requirement |
| 96 | issue | CLOSED | Support for Flutter web? | WEB-ASK | no action |
| 97 | issue | CLOSED | How to access the key from different pages? | NOA-MISC | no action |
| 98 | issue | CLOSED | Default Insecure Data Storage | DOCS | requirement |
| 99 | issue | CLOSED | Is it support for web dev ? | WEB-ASK | no action |
| 100 | issue | CLOSED | BUG: Severe crash | CRASH-MISC | requirement |
| 101 | PR | MERGED | Fix build warning. Issue #30 | NA-LINT | not applicable |
| 102 | issue | CLOSED | iOS autodelete key values items after uninstall? | APP-BACKUP | requirement |
| 103 | issue | CLOSED | AndroidX incompatibility? | NA-BUILD | not applicable |
| 104 | issue | CLOSED | Migrate to v2 Android Plugin API | NA-EMB | not applicable |
| 105 | issue | CLOSED | Android doesn't persist values? | LOSS-2 | requirement |
| 106 | issue | CLOSED | Add support for MacOS Keychain | NOA-MISC | no action |
| 107 | issue | CLOSED | iOSOptions with null group fails to add key to keychain | IOS-QUERY-2 | requirement |
| 108 | PR | MERGED | Support for specifying keychain accessibility options | APP-OPTS | requirement |
| 109 | issue | CLOSED | A problem occurred configuring root project 'flutter\_secure\_storage'. | AND-PKG | not applicable |
| 110 | PR | MERGED | \[flutter\_secure\_storage\] Migrate flutter\_secure\_storage to Android v2 embedder. | AND-EMB | not applicable |
| 111 | issue | CLOSED | License infringement | NOA-MISC | no action |
| 112 | issue | CLOSED | How to protect data from another app | NOA-MISC | no action |
| 113 | issue | CLOSED | Upgrading from v3.3.1 to v3.3.2 has breaking changes for iOSOptions FlutterSecureStorage | LEGACY-VERSION | avoided by design |
| 114 | issue | CLOSED | How can read from a different app? | NOA-MISC | no action |
| 115 | issue | CLOSED | The plugin flutter\_secure\_storage could not be built due to the issue above. | AND-PKG | not applicable |
| 116 | issue | CLOSED | Initial key generation hangs UI rendering | BLOCK | requirement |
| 117 | PR | MERGED | Fix for issue #116: Android hanging UI on StorageCipher initialization | ORDER-2 | requirement |
| 118 | issue | CLOSED | Android 4.4.2: MissingPluginException(No implementation found for method write on cannel plugins.it\_nomads.com | FLUTTER-WIRE | not applicable |
| 119 | issue | CLOSED | Delay reading and showing Data on UI. | ORDER-2 | requirement |
| 120 | issue | CLOSED | I found a problem with recovered application. | AND-BACKUP | requirement |
| 121 | issue | CLOSED | Compile error regarding PR: https://github.com/mogol/flutter\_secure\_storage/commit/a2ab156da02397a76b4ff9e1386 | FLUTTER-WIRE | not applicable |
| 122 | PR | MERGED | remove unnecessary nonnull syntax | NA-LINT | not applicable |
| 123 | issue | CLOSED | LifeSpan functionality | FEATURE-2 | no action |
| 124 | PR | MERGED | Forward fix crash only observed for v2 apps. | NA-EMB | not applicable |
| 125 | issue | CLOSED | Data Persists after app deletion | APP-BACKUP | requirement |
| 126 | issue | CLOSED | Support Background Isolates | FEATURE | no action |
| 127 | issue | CLOSED | Feature Request: containsKey | CONTAINS | requirement |
| 128 | issue | CLOSED | Reading bool from another app | NOA-MISC | no action |
| 129 | issue | CLOSED | Not Working in android 5.1.1 and 4.4.4 | ANDROID-MINAPI | avoided by design |
| 130 | issue | CLOSED | Testing with flutter\_secure\_storage | FLUTTER-WIRE | not applicable |
| 131 | issue | CLOSED | RSACipher fails to generate keys when phone locale is Arabic in android | ANDROID-RSA | avoided by design |
| 132 | PR | MERGED | Fixed crash when generating keys in android with RTL locales | ANDROID-RSA | avoided by design |
| 133 | PR | MERGED | Make MethodResultWrapper static and methodResult final | NA-LINT | not applicable |
| 134 | PR | MERGED | Return the error as String rather than Exception | NA-LINT | not applicable |
| 135 | PR | MERGED | Fix Double-Checked Locking using volatile | NATIVE-MEMORY | avoided by design |
| 136 | issue | CLOSED | Can't run flutter test due to MissingPluginException in flutter\_secure\_storage | FLUTTER-WIRE | not applicable |
| 137 | PR | MERGED | Minor fix: prevent rtl bug fix from permanently changing locale | ANDROID-RSA | avoided by design |
| 138 | issue | CLOSED | Data does not be saved after uninstalling app | AND-BACKUP | requirement |
| 139 | PR | MERGED | Add support for containsKey | CONTAINS | requirement |
| 140 | issue | CLOSED | How does this package store things on the Phone locally? | NOA-MISC | no action |
| 141 | issue | CLOSED | Does not work on Web app | WEB-ASK | no action |
| 142 | issue | CLOSED | Cannot read secure\_storage in native App | NOA-MISC | no action |
| 143 | issue | CLOSED | exception in rtl locale , can not parse date time formate in init | FLUTTER-WIRE-2 | not applicable |
| 144 | issue | CLOSED | \[Android\] Application crashes when initialization fails | AND-LOAD | requirement |
| 145 | PR | MERGED | Fix Android crash onDetachedFromEngine when init fails | CRASH-2 | requirement |
| 146 | issue | CLOSED | \[Request\] Please make it available for web too. | WEB-ASK | no action |
| 147 | issue | CLOSED | doesn't work on real device or emulator if i don't run cmd "flutter run" | FLUTTER-WIRE | not applicable |
| 148 | issue | CLOSED | Unsupported operation: Platform.\_operatingSystem | FLUTTER-WIRE | not applicable |
| 149 | issue | CLOSED | \[Request\] Please make it possible to read and write more variable types | FEATURE | no action |
| 150 | PR | MERGED | Fix AndroidManifest.xml syntax bug | NA-BUILD | not applicable |
| 151 | PR | CLOSED | Fix RSACipher18Implementation bug on startup at API level 19 and lower | ANDROID-RSA | avoided by design |
| 152 | issue | CLOSED | MissingPluginException - lets fix this | FLUTTER-WIRE | not applicable |
| 153 | issue | CLOSED | Add support for Windows | NOA-MISC | no action |
| 154 | issue | CLOSED | Getting Platform exception error after upgraded to flutter 1.20.0 | FLUTTER-WIRE | not applicable |
| 155 | issue | CLOSED | Delete operation in iOS is not permitted? | IOS-QUERY-2 | requirement |
| 156 | issue | CLOSED | How to delete keychain data once the app is uninstalled? | APP-BACKUP | requirement |
| 157 | issue | CLOSED | \[Android\] Apps can't open after update security path on android device | CRYPTO-LEGACY | avoided by design |
| 158 | issue | CLOSED | Unable to execute Dex | NA-BUILD | not applicable |
| 159 | issue | CLOSED | StorageCipher initialization failed | AND-READ | requirement |
| 160 | PR | MERGED | Remove unused dart:async import | NA-LINT | not applicable |
| 161 | issue | CLOSED | Unhandled Exception: PlatformException(Exception encountered, read, javax.crypto.BadPaddingException: error:1e | CRYPTO-LEGACY | avoided by design |
| 162 | issue | CLOSED | Deprecation warning during compilation on Android 10 (API 29) | NA-LINT | not applicable |
| 163 | issue | CLOSED | WRONG\_FINAL\_BLOCK\_LENGTH | CRYPTO-LEGACY | avoided by design |
| 164 | issue | CLOSED | Storage sometimes returns random data | CRYPTO-LEGACY-2 | avoided by design |
| 165 | PR | MERGED | Fixes thread safety issues in android code. closes #161 | INIT-SERIAL-2 | requirement |
| 166 | issue | CLOSED | minSdkVersion | AND-PKG | not applicable |
| 167 | issue | CLOSED | A problem occurred configuring project ':flutter\_secure\_storage'. | AND-PKG | not applicable |
| 168 | issue | CLOSED | java.lang.IllegalStateException | CRASH-2 | requirement |
| 169 | issue | CLOSED | generateKeyInternal failed on request -68 | AND-READ | requirement |
| 170 | PR | CLOSED | make it a singleton class | INIT-SERIAL-2 | requirement |
| 171 | issue | CLOSED | Performance limits on Android | LARGE | requirement |
| 172 | issue | CLOSED | Numerous deprecations | NA-LINT | not applicable |
| 173 | issue | CLOSED | item stored in secure storage is returning null after a period of time | AND-LOSS | requirement |
| 174 | issue | CLOSED | Exception on Xiaomi devices after release to google store v3.3.5, 3.3.4 | CRYPTO-LEGACY-2 | avoided by design |
| 175 | issue | CLOSED | StorageCipher initialization failed on API 27 | AND-READ | requirement |
| 176 | issue | CLOSED | Can this be used for flutter web? | WEB-ASK | no action |
| 177 | PR | MERGED | Fix Locale & KeyPairGeneratorSpec deprecation warnings | NA-LINT | not applicable |
| 178 | issue | CLOSED | Access flutter secure storage values from native android app service | NOA-MISC | no action |
| 179 | issue | CLOSED | Values not persisted when application closed | AND-LOSS | requirement |
| 180 | issue | CLOSED | Add the ability to query wether StorageCipher is initialized | NOA-MISC | no action |
| 181 | PR | CLOSED | Added macOS support. | APP-PKG | not applicable |
| 182 | issue | CLOSED | deprecated API | NA-LINT | not applicable |
| 183 | issue | CLOSED | MissingPluginException(No implementation found for method write on channel plugings.it\_nomads.com/flutter\_secu | FLUTTER-WIRE | not applicable |
| 184 | PR | MERGED | Make it possible to have no iOS account name(kSecAttrService) | NAMES-R11-2 | requirement |
| 185 | PR | MERGED | Added support for Linux | NOA-MISC | no action |
| 186 | issue | CLOSED | is flutter secure storage null safe? | NOA-MISC | no action |
| 187 | PR | MERGED | null safety | NOA-MISC | no action |
| 188 | issue | CLOSED | Keychain storage duplicate key value. | APP-UPSERT | requirement |
| 189 | issue | CLOSED | Was working but without any change, Null Pointers | CRASH-2 | requirement |
| 190 | issue | CLOSED | Secure Storage was working but now Null Pointer-ing | CRASH-2 | requirement |
| 191 | issue | CLOSED | StorageCipher initialization failed on Android 11 (Pixel 5) | AND-READ | requirement |
| 192 | issue | CLOSED | How long does this plugin stores data | NOA-MISC | no action |
| 193 | issue | CLOSED | PlatformException after updating the application in google play store. | CRYPTO-LEGACY | avoided by design |
| 194 | PR | MERGED | Improve first-time read speed on Android by not creating cipher when key is not present | PERF-2 | requirement |
| 195 | issue | CLOSED | App crashes when doing any secure storage read or write operation on IOS simulator - No exception logged | CRASH-MISC | requirement |
| 196 | issue | CLOSED | \[Question\]: Does this plugin create a separate secure storage for each app? | NOA-MISC | no action |
| 197 | issue | CLOSED | Depreciated API warning | NA-LINT | not applicable |
| 198 | issue | CLOSED | Old Encryption Format - .setBlockModes("ECB") - Android | CRYPTO-LEGACY | avoided by design |
| 199 | issue | CLOSED | How to show all items? | NOA-MISC | no action |
| 200 | issue | CLOSED | Getting Platform Exception on writing data after updating the app | CRYPTO-LEGACY | avoided by design |
| 201 | issue | CLOSED | StorageCipher initialization failed | AND-READ | requirement |
| 202 | issue | CLOSED | Why not use apply instead of commit? | NOA-MISC | no action |
| 203 | issue | CLOSED | Use EncryptedSharedPreferences instead of SharedPreferences for Android | CRYPTO-LEGACY | avoided by design |
| 204 | PR | CLOSED | #203 Use EncryptedSharedPreferences instead of raw preferences | CRYPTO-LEGACY | avoided by design |
| 205 | issue | CLOSED | flutter\_secure\_storage package and AppStore Export Compliance | NOA-MISC | no action |
| 206 | issue | CLOSED | Application crashes since yesterday on android-arm64 • Android 11 (API 30) | CRASH-2 | requirement |
| 207 | issue | CLOSED | Attempt to invoke virtual method 'void d.a.b.a.j.a(d.a.b.a.j$c)' on a null object reference | AND-LOAD | requirement |
| 208 | issue | CLOSED | Null check operator used on a null value on read method | FLUTTER-WIRE | not applicable |
| 209 | issue | CLOSED | Could not find class 'android.security.keystore.KeyGenParameterSpec$Builder' | AND-LOAD | requirement |
| 210 | issue | CLOSED | Unhandled Exception: PlatformException(Exception encountered, read, javax.crypto.BadPaddingException: error:1e | CRYPTO-LEGACY | avoided by design |
| 211 | issue | CLOSED | Need help: Migration from existing app to flutter | LEGACY-VERSION | avoided by design |
| 212 | issue | CLOSED | Error on reading data | NOA-MISC | no action |
| 213 | issue | CLOSED | Linux Desktop Build Failed: cannot find -lpkgcfg\_lib\_LIBSECRET\_secret-1-NOTFOUND | LINUX-BUILD | not applicable |
| 214 | issue | CLOSED | Read: key exists =\> Running ensureInitStorageCipher / StorageCipher already initialized | ANDROID-INIT-LOOP | avoided by design |
| 215 | issue | CLOSED | compatibility with the web | WEB-ASK | no action |
| 216 | issue | CLOSED | Flutter 2 | FLUTTER-WIRE | not applicable |
| 217 | PR | MERGED | Removed the debug logging from Android side of plugin. | LOG-2 | requirement |
| 218 | issue | CLOSED | Storing JWT token returns null?! | ORDER-2 | requirement |
| 219 | issue | CLOSED | Null check operator used on a null value | FLUTTER-WIRE | not applicable |
| 220 | issue | CLOSED | SDK location not found after upgrading to Flutter 2.0 | NA-BUILD | not applicable |
| 221 | issue | CLOSED | Instance of 'Mo' | FLUTTER-WIRE | not applicable |
| 222 | issue | CLOSED | Do you support Windows? | NOA-MISC | no action |
| 223 | issue | CLOSED | Flutter secure storage data is getting cleared on IOS when the app is in background more than 2 hr. | APP-LOCKED | requirement |
| 224 | issue | CLOSED | how can I read a value I have set in my flutter app? | NOA-MISC | no action |
| 225 | PR | MERGED | Removed the Strongbox support as it turns out to be unstable. | FEATURE-2 | no action |
| 226 | issue | CLOSED | Read: key exists =\> Running ensureInitStorageCipher StorageCipher already initialized | ANDROID-INIT-LOOP | avoided by design |
| 227 | issue | CLOSED | Flutter run on Linux gives A required package was not found | LINUX-BUILD | not applicable |
| 228 | issue | CLOSED | Clear down stored entries out side of the application on Linux Desktop | LINUX-DOC | requirement |
| 229 | issue | CLOSED | No implementation found for method READ and WRITE in Flutter Secure Storage | FLUTTER-WIRE | not applicable |
| 230 | issue | CLOSED | Export iOS native methods | APP-PKG | not applicable |
| 231 | issue | CLOSED | Read: key exists =\> Running ensureInitStorageCipher and StorageCipher already initialized infinite logs | ANDROID-INIT-LOOP | avoided by design |
| 232 | issue | CLOSED | Unable to ship with a snap version | LINUX-BUILD | not applicable |
| 233 | issue | CLOSED | Flutter run on linux cannot link secret library | LINUX-BUILD | not applicable |
| 234 | issue | CLOSED | FlutterSecureStoragePlugin.java uses or overrides a deprecated API. | AND-PKG | not applicable |
| 235 | issue | CLOSED | \[Question\] \[Android\] Can this plugin be used in the background while the phone screen is off? | NOA-MISC | no action |
| 236 | issue | CLOSED | \[ERROR:flutter/lib/ui/ui\_dart\_state.cc(199)\] Unhandled Exception: Null check operator used on a null value | FLUTTER-WIRE | not applicable |
| 237 | issue | CLOSED | error: package com.it\_nomads.fluttersecurestorage does not exist | AND-PKG | not applicable |
| 238 | issue | CLOSED | Android Keystore entry is deleted when app is uninstalled? | AND-BACKUP | requirement |
| 239 | issue | CLOSED | Flutter secure storage not storing once app is closed | AND-LOSS | requirement |
| 240 | PR | MERGED | \[iOS\] kSecAttrSynchronizable support | APP-SYNC | avoided by design |
| 241 | issue | CLOSED | Unable to access keychain values from prior native app | NOA-MISC | no action |
| 242 | issue | CLOSED | WIndows support | NOA-MISC | no action |
| 243 | issue | CLOSED | In iOS Data Persists after uninstalling the application | APP-BACKUP | requirement |
| 244 | PR | MERGED | docs: added info on IOSOptions | DEPS | not applicable |
| 245 | issue | CLOSED | Specify the importance of IOSAccessibility options in the readme | APP-OPTS | requirement |
| 246 | PR | MERGED | Replace jcenter with mavenCentral | NA-BUILD | not applicable |
| 247 | PR | CLOSED | Rough windows implementation of secure storage | NOA-MISC | no action |
| 248 | issue | CLOSED | PlatformException(Exception encountered, read, javax.crypto.BadPaddingException: error:1e000065:Cipher functio | CRYPTO-LEGACY | avoided by design |
| 249 | issue | CLOSED | CBC Ciphers | CRYPTO-LEGACY | avoided by design |
| 250 | issue | CLOSED | Repository maintainer | NOA-MISC | no action |
| 251 | issue | CLOSED | Cannot build appbundle with targetSdkVersion 31 | AND-PKG | not applicable |
| 252 | issue | CLOSED | Web Now Possible and secure! | WEB-ASK | no action |
| 253 | issue | CLOSED | Use new linter | AND-PKG | not applicable |
| 254 | issue | CLOSED | Use Github actions to check for formatting and analysis errors | NA-LINT | not applicable |
| 255 | PR | CLOSED | Added macOS support based on #181 | APP-PKG | not applicable |
| 256 | issue | CLOSED | Migrate existing objc code to Swift | APP-PKG | not applicable |
| 257 | PR | MERGED | Update to use platform interface and implement web, windows, macOS and split Linux into it's own project per b | NA-BUILD | not applicable |
| 258 | issue | CLOSED | java.security.UnrecoverableKeyException: Failed to obtain X.509 form of public key | AND-READ | requirement |
| 259 | PR | MERGED | #203 encrypted shared preferences | CRYPTO-LEGACY | avoided by design |
| 260 | PR | MERGED | release of v4.2.1 | DEPS | not applicable |
| 261 | PR | MERGED | hotfix: remove gitignored files | LAST-DEPS | not applicable |
| 262 | PR | MERGED | imp: changed to endorsed plugin | NA-BUILD | not applicable |
| 263 | issue | CLOSED | Testers and developers wanted for Linux, Windows, macOS & web ! | WEB-ASK | no action |
| 264 | issue | CLOSED | Deleting key not working on macOS | APP-QUERY | requirement |
| 265 | PR | MERGED | imp: removed old embedding, added null checks and type casted maps, c… | NA-EMB | not applicable |
| 266 | issue | CLOSED | \[BLOCKING\] No implementation for containsKey on iOS, macOs, linux | CONTAINS | requirement |
| 267 | issue | CLOSED | 5.0.0-beta.1 Android Errors on readAll with previous values from 4.x.x with default settings | CRYPTO-LEGACY | avoided by design |
| 268 | issue | CLOSED | 5.0.0-beta.1 Android - Note: flutter\_secure\_storage-5.0.0-beta.1\android\src\main\java\com\it\_nomads\flutterse | CRYPTO-LEGACY | avoided by design |
| 269 | issue | CLOSED | EncryptedSharedPreferences implementation could be better | CRYPTO-LEGACY | avoided by design |
| 270 | PR | MERGED | 269 encrypted shared pref improvement | LAST-ESP | avoided by design |
| 271 | PR | MERGED | Fixes Secure Storage Web Bug on containsKey and implements a containsKey option in the example app that could | CONTAINS | requirement |
| 272 | issue | CLOSED | Unable to write to secure storage after updating the package from 4.2.0 to 4.2.1 (ANDROID)(RELEASE APK) | CRYPTO-LEGACY | avoided by design |
| 273 | PR | MERGED | Implement containsKey on Android | CONTAINS | requirement |
| 274 | PR | MERGED | Windows implementations of containsKey, readAll and deleteAll | CONTAINS | requirement |
| 275 | issue | CLOSED | Incorrect write implementation on Windows | WIN-FILE | requirement |
| 276 | issue | CLOSED | \_CastError (Null check operator used on a null value) | FLUTTER-WIRE | not applicable |
| 277 | issue | CLOSED | Windows applications crash after calling storage.read | MISSING | requirement |
| 278 | issue | CLOSED | 6.0.0 Add migration from secure storage to EncryptedSharedPreference | CRYPTO-LEGACY | avoided by design |
| 279 | issue | CLOSED | \[Feature\] Add resetOnError parameter to reinitialize data in case of unrecoverable key | AND-RESET | requirement |
| 280 | PR | MERGED | feature: add possibility to reset data on Android on exception | AND-RESET | requirement |
| 281 | PR | MERGED | Add option defaults at the overall class | NOA-MISC | no action |
| 282 | issue | CLOSED | Migrating from electron node-keytar to flutter\_secure\_storage | NOA-MISC | no action |
| 283 | issue | CLOSED | m1 ,chrome ,show error "Error: Unsupported operation: Platform.\_operatingSystem | FLUTTER-WIRE | not applicable |
| 284 | issue | CLOSED | Keychain items lost after app update installed from App Store | IOS-QUERY-2 | requirement |
| 285 | issue | CLOSED | Read a nonexists key cause an error "No implementation found for method read on channel plugins.it\_nomads.com/ | MISSING | requirement |
| 286 | issue | CLOSED | 5.0.0-beta.3 fails to build on linux | LINUX-BUILD | not applicable |
| 287 | issue | CLOSED | Windows read crashes if key doesn't exist yet | MISSING | requirement |
| 288 | PR | MERGED | fix read crash when key doesn't exist | MISSING | requirement |
| 289 | issue | CLOSED | No implementation found for method write on channel plugins.it\_nomads.com/flutter\_secure\_storage | FLUTTER-WIRE | not applicable |
| 290 | issue | CLOSED | flutter\_secure\_storage\_windows doesn't work | NOA-MISC | no action |
| 291 | issue | CLOSED | \[iOS\] Not working when published on TestFlight | APP-ENT-2 | requirement |
| 292 | PR | MERGED | Fix delete key on web | WEB-DELETE | requirement |
| 293 | issue | CLOSED | SecItemAdd status = -34018 | APP-STATUS | requirement |
| 294 | issue | CLOSED | BIG ISSUE ( WINDOWS ) : It shares the data between two apps with the same key | WIN-SHARE | requirement |
| 295 | PR | MERGED | \[flutter\_secure\_storage\_linux\] Fixes #286 and add support for containsKey in linux | CONTAINS | requirement |
| 296 | issue | CLOSED | Data Types to store | FEATURE | no action |
| 297 | PR | MERGED | feat: add containsKey to macOS & iOS | CONTAINS | requirement |
| 298 | issue | CLOSED | error: use of undeclared identifier 'flutter\_secure\_storage\_linux\_plugin\_register\_with\_registrar' on Linux | LINUX-BUILD | not applicable |
| 299 | issue | CLOSED | Module 'flutter\_secure\_storage' not found | NA-BUILD | not applicable |
| 300 | issue | CLOSED | \[WEB\] Unable to read keys | WEB-CONTEXT-2 | requirement |
| 301 | issue | CLOSED | 5.0.0.beta-4-5 Android fails to build any project that depends on this with: Could not initialize class sun.se | AND-PKG | not applicable |
| 302 | issue | CLOSED | macOS - containsKey is always returning true | CONTAINS | requirement |
| 303 | issue | CLOSED | IllegalStateException on a device with no pin/security | CRASH-MISC | requirement |
| 304 | issue | CLOSED | StorageCipher initialization failed | AND-READ | requirement |
| 305 | issue | CLOSED | Linux appears to be broken on 5.0.0-beta.5 branch | LINUX-BUILD | not applicable |
| 306 | issue | CLOSED | 2 different apps read the value store by other app | WIN-SHARE | requirement |
| 307 | issue | CLOSED | Windows issue on writing or reading values | WIN-FILE | requirement |
| 308 | issue | CLOSED | Flutter app gets stuck on white screen when opening SAMSUNG A205F (flutter\_secure\_storage: ^4.2.0) | CRYPTO-LEGACY-2 | avoided by design |
| 309 | issue | CLOSED | Remote error from secret service: org.freedesktop.DBus.Error.AccessDenied: An AppArmor policy prevents this se | LINUX-2 | requirement |
| 310 | issue | CLOSED | \[Question\] \[iOS\] How to reset secure storage on iOS from app settings ? | NOA-MISC | no action |
| 311 | issue | CLOSED | Firebase Crashlytics Errors | NOA-MISC | no action |
| 312 | issue | CLOSED | \[issue\] \[android\] StorageCipher initialization failed | AND-READ | requirement |
| 313 | issue | CLOSED | 5.0.0-beta.5: deleteAll throws "java.lang.SecurityException: Could not decrypt key. decryption failed" | AND-READ | requirement |
| 314 | issue | CLOSED | build application failed with flutter installed via snap in linux | LINUX-BUILD | not applicable |
| 315 | issue | CLOSED | Deprecation warnings show up during first build | LINUX-BUILD | not applicable |
| 316 | issue | CLOSED | \[Workmanager\]\[iOS\]\[BGProcessingTask\] Return null value when reading key within executeTask's callback | APP-LOCKED | requirement |
| 317 | PR | MERGED | fix: Fixed android language recovery Settings not taking effect | NA-EMB | not applicable |
| 318 | issue | CLOSED | Release Timeline for 5.0.0 | NOA-MISC | no action |
| 319 | issue | CLOSED | Build error for linux 5.0 beta | LINUX-BUILD | not applicable |
| 320 | issue | CLOSED | Runtime error for web 5.0 | NA-BUILD | not applicable |
| 321 | issue | CLOSED | fatal error: 'Flutter/Flutter.h' file not found | APP-PKG | not applicable |
| 322 | issue | CLOSED | How safe is this package for storing ethereum private keys in the device? | DOCS | requirement |
| 323 | issue | CLOSED | Calling secure storage from getters and setters | NOA-MISC | no action |
| 324 | issue | CLOSED | Unable to read existing value in iOS keychain | IOS-QUERY-2 | requirement |
| 325 | PR | MERGED | \[windows\] Make credential key unique to the app | WIN-SHARE | requirement |
| 326 | PR | MERGED | \[windows\] Silently ignore errors when deleting keys that don't exist | MISSING | requirement |
| 327 | PR | MERGED | ci: add new format & linter options, enable flutter\_drive | DEPS | not applicable |
| 328 | PR | MERGED | Android encryption migration | CRYPTO-LEGACY-2 | avoided by design |
| 329 | PR | CLOSED | release of v5.0.0 | DEPS | not applicable |
| 330 | issue | CLOSED | v5.0.0: error: lambda expressions and method references are not supported in -source 7 (use -source 8 or highe | AND-PKG | not applicable |
| 331 | PR | MERGED | bug: fix java 8 requirement | NA-BUILD | not applicable |
| 332 | issue | CLOSED | Cannot read from SharedPreferences after updating to version 5.0.0 | CRYPTO-LEGACY | avoided by design |
| 333 | issue | CLOSED | Unable to build android apk after updating to v5.0.0 | AND-PKG | not applicable |
| 334 | PR | MERGED | bug: fix sharedPrefs not being initialized | INIT-SERIAL-2 | requirement |
| 335 | issue | CLOSED | Flutter Secure Storage Issues: Unable to read or write keys and values | NOA-MISC | no action |
| 336 | issue | CLOSED | "Null check operator used on null value" error on read | FLUTTER-WIRE | not applicable |
| 337 | issue | CLOSED | Unable to .readAll() keys on Android 9 (API 28) | ORDER-2 | requirement |
| 338 | issue | CLOSED | containsKey always returning true | CONTAINS | requirement |
| 339 | issue | CLOSED | Java compiler error on version 5 of flutter\_secure\_storage | AND-PKG | not applicable |
| 340 | issue | CLOSED | IOSAccessibility.passcode | APP-OPTS | requirement |
| 341 | issue | CLOSED | Add api to read/writ bytes | FEATURE | no action |
| 342 | issue | CLOSED | StrictMode disk read violation | ORDER-2 | requirement |
| 343 | issue | CLOSED | Values not updating right away | ORDER | requirement |
| 344 | issue | CLOSED | Android embedding v2 | AND-EMB | not applicable |
| 345 | issue | CLOSED | UNKNOWN\_ERROR on windows for large secrets (\>2561 bytes) | WIN-SIZE | avoided by design |
| 346 | issue | CLOSED | read() working weird after upgrading to iOS 15.1 | APP-QUERY | requirement |
| 347 | issue | CLOSED | DeleteAll crashing for some users | AND-READ | requirement |
| 348 | issue | CLOSED | Error Flutter/Flutter.h | APP-PKG | not applicable |
| 349 | issue | CLOSED | Should I say that "my app uses encryption" if I use this library? | NOA-MISC | no action |
| 350 | issue | CLOSED | Mac OS write read doesn't work | APP-QUERY | requirement |
| 351 | issue | CLOSED | Secure storage not replicated across devices. | APP-SYNC | avoided by design |
| 352 | issue | CLOSED | 2nd time restart application flush storage | AND-LOSS | requirement |
| 353 | issue | CLOSED | Linux read always return null | LINUX-KEYRING | requirement |
| 354 | issue | CLOSED | Unhandled Exception: PlatformException(Exception encountered, read, javax.crypto.BadPaddingException: error:1e | CRYPTO-LEGACY | avoided by design |
| 355 | issue | CLOSED | Uncaught TypeError: Cannot read properties of undefined (reading 'generateKey') | WEB-CONTEXT | requirement |
| 356 | issue | CLOSED | include file: 'atlstr.h' is missing | WIN-PKG | not applicable |
| 357 | issue | CLOSED | Can't be able to delete data | AND-LOSS | requirement |
| 358 | issue | CLOSED | Retrieving stored JWT returns NULL after some time. | AND-LOSS | requirement |
| 359 | issue | CLOSED | Android - containsKey is always returning false | CONTAINS | requirement |
| 360 | issue | CLOSED | Desktop Storage | NOA-MISC | no action |
| 361 | issue | CLOSED | Exception has occurred. PlatformException (PlatformException(Exception encountered: UNKNOWN\_ERROR, write, null | WIN-SIZE-2 | avoided by design |
| 362 | issue | CLOSED | Linux Desktop build fails due to linker | LINUX-BUILD | not applicable |
| 363 | issue | CLOSED | Weird exception on first start Android 11 Samsung A51 | KEYSTORE-READ-2 | requirement |
| 364 | PR | MERGED | build(deps): bump gradle from 7.0.3 to 7.1.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 365 | PR | MERGED | build(deps): bump subosito/flutter-action from 1 to 2.3.0 | DEPS | not applicable |
| 366 | PR | CLOSED | build(deps): bump gradle from 7.0.3 to 7.1.1 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 367 | PR | MERGED | Update the docs of FlutterSecureStorage.delete(). | DOCS-2 | requirement |
| 368 | issue | CLOSED | MissingPluginImplementation weird behavior | FLUTTER-WIRE | not applicable |
| 369 | issue | CLOSED | \[bug\] MacOS: Completely not usable - read always gives \*nothing\* | APP-QUERY | requirement |
| 370 | PR | MERGED | Explain in detail what to do for MacOS platform | DOCS-2 | requirement |
| 371 | issue | CLOSED | Secure Storage data persists when uninstalling an app on iOS | APP-BACKUP | requirement |
| 372 | PR | CLOSED | build(deps): bump gradle from 7.0.3 to 7.1.2 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 373 | PR | MERGED | build(deps): bump gradle from 7.1.1 to 7.1.2 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 374 | issue | CLOSED | Android - Exposing the plugin native API to the native part of the app | AND-EMB | not applicable |
| 375 | PR | MERGED | #374 - Expose the Android API used by the Flutter plugin | AND-EMB | not applicable |
| 376 | PR | MERGED | ci: update gradle | DEPS | not applicable |
| 377 | issue | CLOSED | My existing flutter project not running for Linux desktop app | LINUX-BUILD | not applicable |
| 378 | PR | MERGED | build(deps): bump actions/checkout from 2 to 3 | DEPS | not applicable |
| 379 | issue | CLOSED | flutter\_secure\_storage does not work on Windows. | NOA-MISC | no action |
| 380 | issue | CLOSED | Attempt to invoke interface method 'boolean android.content.SharedPreferences.contains(java.lang.String)' | AND-LOAD | requirement |
| 381 | issue | CLOSED | OperationError when calling read() on web | WEB-READ | requirement |
| 382 | issue | CLOSED | Linux desktop issues | LINUX-BUILD | not applicable |
| 383 | issue | CLOSED | CocoaPods error on M1 | APP-PKG | not applicable |
| 384 | issue | CLOSED | Missing license file | NA-BUILD | not applicable |
| 385 | PR | MERGED | build(deps): bump actions/setup-java from 2 to 3 | DEPS | not applicable |
| 386 | PR | CLOSED | build(deps): bump gradle from 7.1.2 to 7.1.3 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 387 | PR | CLOSED | build(deps): bump gradle from 7.1.2 to 7.1.3 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 388 | issue | CLOSED | flutter\_secure\_storage does not work on Windows. | NOA-MISC | no action |
| 389 | issue | CLOSED | Private Browsing in Firefox not working | WEB-CONTEXT | requirement |
| 390 | PR | MERGED | build(deps): bump subosito/flutter-action from 2.3.0 to 2.4.0 | DEPS | not applicable |
| 391 | issue | CLOSED | Data of FlutterSecureStorage instance is null on iOS but not on Android | APP-QUERY | requirement |
| 392 | issue | CLOSED | java.lang.Exception: No certificate found under alias | AND-READ | requirement |
| 393 | issue | CLOSED | Update the gradle please | AND-PKG | not applicable |
| 394 | issue | CLOSED | iOS debug builds are not persisting keychain data after background | APP-LOCKED | requirement |
| 395 | PR | CLOSED | WIP: Migrated to EncryptedSharedPreferences | CRYPTO-LEGACY | avoided by design |
| 396 | PR | CLOSED | build(deps): bump gradle from 7.1.2 to 7.2.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 397 | PR | CLOSED | build(deps): bump gradle from 7.1.2 to 7.2.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 398 | PR | MERGED | fix issue of memory leak | NATIVE-MEMORY | avoided by design |
| 399 | issue | CLOSED | Are custom options supposed to be used also in common operations? | NAMES-R11-2 | requirement |
| 400 | PR | MERGED | Feature/android modern algorithms | CRYPTO-LEGACY | avoided by design |
| 401 | PR | MERGED | build(deps): bump gradle from 7.1.2 to 7.2.1 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 402 | PR | MERGED | build(deps): bump gradle from 7.1.2 to 7.2.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 403 | PR | MERGED | bug: release data to free memory | NATIVE-MEMORY | avoided by design |
| 404 | issue | CLOSED | Uncaught TypeError: Cannot read properties of undefined (reading 'generateKey') | WEB-CONTEXT | requirement |
| 405 | PR | MERGED | Expose macOS Keychain options | OPTS-2 | requirement |
| 406 | issue | CLOSED | Written data removed automatically immediately in MacOs | APP-QUERY | requirement |
| 407 | PR | MERGED | Fix Linux: error not thrown when using secret service | LINUX-VERIFY | requirement |
| 408 | issue | CLOSED | W/AndroidKeysetManager( 5792): java.io.FileNotFoundException: can't read keyset; the pref value \_\_androidx\_sec | CRYPTO-LEGACY | avoided by design |
| 409 | issue | CLOSED | NOT WORKING ON WINDOWS !!!! | NOA-MISC | no action |
| 410 | issue | CLOSED | Web - Error: MissingPluginException(No implementation found for method write on channel plugins.it\_nomads.com/ | WEB-PKG | not applicable |
| 411 | PR | MERGED | Add a note on the C++ ATL requirement for windows | APP-PKG | not applicable |
| 412 | issue | CLOSED | Not working in windows web | WEB-ASK | no action |
| 413 | issue | CLOSED | \[Loophole\] Is it really secure? | DOCS | requirement |
| 414 | issue | CLOSED | EncryptedSharedPreferences initialization failed | CRYPTO-LEGACY | avoided by design |
| 415 | issue | CLOSED | How to choose specific keychain group to get data? | APP-QUERY | requirement |
| 416 | issue | CLOSED | Problem with Mock Unit Test - type 'Null' is not a subtype of type 'Future\<void\>' | FLUTTER-WIRE | not applicable |
| 417 | issue | CLOSED | resetOnError returns String | AUTH-NAMES | avoided by design |
| 418 | issue | CLOSED | Example does not function on Custom embedder | AND-EMB | not applicable |
| 419 | issue | CLOSED | PEN test failed on iOS | NOA-MISC | no action |
| 420 | issue | CLOSED | Add a way to watch/listen changes | FEATURE | no action |
| 421 | issue | CLOSED | the project is not build on Flutter 3.0.5 and Dart 2.17.6 | FLUTTER-WIRE | not applicable |
| 422 | PR | MERGED | build(deps): bump subosito/flutter-action from 2.4.0 to 2.6.1 | DEPS | not applicable |
| 423 | issue | CLOSED | After reinstall the app, the key stored last time was lost | AND-BACKUP | requirement |
| 424 | PR | MERGED | Imp/linter | DEPS | not applicable |
| 425 | issue | CLOSED | change documentation 5.1.0 | LAST-MISC | no action |
| 426 | issue | CLOSED | Unable to build on linux version 5.10.0 | LINUX-BUILD | not applicable |
| 427 | issue | CLOSED | Implement flutter\_secure\_storage for web (All the precautions) | WEB-ASK | no action |
| 428 | issue | CLOSED | StandardMethodCodec.decodeEnvelope | FLUTTER-WIRE | not applicable |
| 429 | PR | MERGED | build(deps): bump gradle from 7.2.1 to 7.2.2 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 430 | PR | MERGED | build(deps): bump gradle from 7.2.1 to 7.2.2 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 431 | issue | CLOSED | Major version bump for Android SDK upgrades | AND-PKG | not applicable |
| 432 | issue | CLOSED | pub.dev Unverified uploader & Unknown Platforms | FLUTTER-WIRE | not applicable |
| 433 | issue | CLOSED | Static linking of libjsoncpp | LINUX-BUILD | not applicable |
| 434 | issue | CLOSED | I Got Error On Iphone When Put String, After App Run 1 Weeks | APP-QUERY | requirement |
| 435 | issue | CLOSED | Deprecated API usage | AND-PKG | not applicable |
| 437 | PR | MERGED | Imp/swift conversion | APP-PKG | not applicable |
| 438 | issue | CLOSED | false format of saving the key on linux | LINUX-KEYRING | requirement |
| 439 | issue | CLOSED | Does this issue closed? | NOA-MISC | no action |
| 440 | issue | CLOSED | Warning: The plugin flutter\_secure\_storage requires Android SDK version 32. | AND-PKG | not applicable |
| 441 | issue | CLOSED | Possible issue where values are deleted from storage on newer versions? | AND-LOSS | requirement |
| 442 | issue | CLOSED | Android Implementation - Make read and write methods public | AND-EMB | not applicable |
| 443 | PR | CLOSED | build(deps): bump subosito/flutter-action from 2.6.1 to 2.6.2 | DEPS | not applicable |
| 444 | issue | CLOSED | 2 exceptions preventing use | NOA-MISC | no action |
| 445 | issue | CLOSED | App cannot build with package. Android dependency error | AND-PKG | not applicable |
| 446 | issue | CLOSED | RSACipher18Implementation.java uses or overrides a deprecated API | AND-PKG | not applicable |
| 447 | issue | CLOSED | web dont work - ver. 5.2.1/6.0.0 | WEB-ASK | no action |
| 448 | issue | CLOSED | registerWith | AND-EMB | not applicable |
| 449 | issue | CLOSED | addPrefixToKey called before ELEMENT\_PREFERENCES\_KEY\_PREFIX is set | AND-STORE-ID | requirement |
| 450 | issue | CLOSED | \[Windows\] Unable to store data more than 2559 characters. Errors with "UNKNOWN ERROR" | WIN-SIZE | avoided by design |
| 451 | PR | CLOSED | build(deps): bump subosito/flutter-action from 2.6.1 to 2.7.0 | DEPS | not applicable |
| 452 | PR | MERGED | build(deps): bump gradle from 7.2.2 to 7.3.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 453 | PR | MERGED | build(deps): bump gradle from 7.2.2 to 7.3.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 454 | PR | MERGED | build(deps): bump subosito/flutter-action from 2.6.1 to 2.7.1 | DEPS | not applicable |
| 455 | issue | CLOSED | Streaming a value | FEATURE | no action |
| 456 | issue | CLOSED | \[iOS 16\]FlutterSecureStorage.containsKey returns true when FlutterSecureStorage. is empty | CONTAINS | requirement |
| 457 | issue | CLOSED | \[Android\] At app start, value is returned only after second read | ORDER | requirement |
| 458 | issue | CLOSED | \[Windows\] error building the app with plugin | WIN-PKG | not applicable |
| 459 | issue | CLOSED | Verify on pub.dev | FLUTTER-WIRE | not applicable |
| 460 | issue | CLOSED | \[Android\] Stacktraces of exceptions should be reported via stacktrace property of PlatformException | LOG | requirement |
| 461 | PR | MERGED | Update CHANGELOG.md | DEPS | not applicable |
| 462 | issue | CLOSED | Wrong type for groupId parameter on iOS native implementation | APP-OPTS | requirement |
| 463 | issue | CLOSED | gnome-keyring requirement for Linux not mentioned in README | LINUX-DOC | requirement |
| 464 | issue | CLOSED | libsecret\_error: Failed to execute child process | LINUX-KEYRING | requirement |
| 465 | PR | MERGED | groupId type updated | APP-QUERY | requirement |
| 466 | PR | MERGED | build(deps): bump subosito/flutter-action from 2.7.1 to 2.8.0 | DEPS | not applicable |
| 467 | PR | MERGED | build(deps): bump gradle from 7.3.0 to 7.3.1 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 468 | PR | MERGED | build(deps): bump gradle from 7.3.0 to 7.3.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 469 | issue | CLOSED | \[macOS\] macOS app does not launch after keychain sharing capability was selected | APP-STATUS | requirement |
| 470 | PR | MERGED | Add setMockInitialValues to enable test | FLUTTER-WIRE | not applicable |
| 471 | issue | CLOSED | Make the library test friendly | FLUTTER-WIRE | not applicable |
| 472 | issue | CLOSED | Unsafe Encryption org.jmrtd.protocol.PACEProtocol.pseudoRandomFunction | CRYPTO-LEGACY | avoided by design |
| 473 | PR | CLOSED | dependencies version updated to latest version | LAST-DEPS | not applicable |
| 474 | issue | CLOSED | Add possibility to invalidate storage after biometric changes | AND-BIO-KEY | requirement |
| 475 | issue | CLOSED | Attention point related to "Padding Oracle", reported by MobSF, when using encryption mode, "AES/CBC/PKCS7Padd | CRYPTO-LEGACY | avoided by design |
| 476 | PR | MERGED | build(deps): bump security-crypto from 1.1.0-alpha03 to 1.1.0-alpha04 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 477 | issue | CLOSED | error in android | NOA-MISC | no action |
| 478 | PR | MERGED | test: update sdk | DEPS | not applicable |
| 479 | PR | MERGED | doc: add docs | DOCS-2 | requirement |
| 480 | PR | MERGED | imp: fix android deprecation warnings | NA-LINT | not applicable |
| 481 | PR | CLOSED | release of v6.1.0 | DEPS | not applicable |
| 482 | issue | CLOSED | iOS persistent local storage data after reinstall | APP-BACKUP | requirement |
| 483 | issue | CLOSED | macOS error | NOA-MISC | no action |
| 484 | issue | CLOSED | \[iOS\] \[6.0.1\] crash in combination with hydrated\_bloc and activated flightmode | CRASH-MISC | requirement |
| 485 | PR | MERGED | Bug/contains key | CONTAINS | requirement |
| 486 | issue | CLOSED | macOS build fails with error cannot find GeneratedPluginRegistrant.swift:12:3 | APP-PKG | not applicable |
| 487 | issue | CLOSED | v7.0.0 Upgraded from v6.0.0 is not working \[Tested on Android Device Only\] | LEGACY-VERSION | avoided by design |
| 488 | PR | CLOSED | add GCM instead of CBC | CRYPTO-LEGACY | avoided by design |
| 489 | PR | MERGED | Fix macOS plugin naming | APP-PKG | not applicable |
| 490 | issue | CLOSED | \[Android\] Is it possible to write data from one app, and read from another? | NOA-MISC | no action |
| 491 | PR | MERGED | Update README.md | DEPS | not applicable |
| 492 | PR | MERGED | Update README | DEPS | not applicable |
| 493 | issue | CLOSED | \[macOS\] Cannot compile 7.0.x for macOS because of Plugin Renaming | APP-PKG | not applicable |
| 494 | issue | CLOSED | \[iOS\] Set access control flags through IOSOptions | APP-OPTS | requirement |
| 495 | PR | MERGED | Linux: fix memory management issues | NATIVE-MEMORY | avoided by design |
| 496 | issue | CLOSED | Rewrite (write on existing value) value not working | UPSERT-2 | requirement |
| 497 | PR | CLOSED | Fix incorrect spelling in CHANGELOG file | DEPS | not applicable |
| 498 | PR | MERGED | Fix spelling in CHANGELOG | DEPS | not applicable |
| 499 | issue | CLOSED | Flutter build fails on macos | APP-PKG | not applicable |
| 500 | issue | CLOSED | Could not find com.android.tools.build:gradle:7.3.1. | AND-PKG | not applicable |
| 501 | issue | CLOSED | v7.0.1 PlatformException(Exception encountered, readAll, java.lang.SecurityException: Could not decrypt key. d | AND-READ | requirement |
| 502 | issue | CLOSED | keyset not found, will generate a new one. can't read keyset; the pref value \_\_androidx\_security\_crypto\_encryp | CRYPTO-LEGACY | avoided by design |
| 503 | issue | CLOSED | I am facing issues while building for linux | LINUX-BUILD | not applicable |
| 504 | issue | CLOSED | \[Question\] Do I need to make the key unique among different APPs? | NOA-MISC | no action |
| 505 | PR | MERGED | \[Windows\] Write encrypted data to files instead of the windows credential system | WIN-SIZE | avoided by design |
| 506 | PR | MERGED | build(deps): bump gradle from 7.3.1 to 7.4.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 507 | PR | MERGED | build(deps): bump gradle from 7.3.1 to 7.4.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 508 | PR | MERGED | use preferences fallback when encryption fails | AND-FALLBACK | avoided by design |
| 509 | issue | CLOSED | \[Question\] Web private key storage | WEB-ASK | no action |
| 510 | issue | CLOSED | \[Question\] Will there ever be synchronous functions | WEB-ASK | no action |
| 511 | issue | CLOSED | Android: IllegalAcessError on flutter\_secure\_storage v7.0.\* | LINUX-BUILD | not applicable |
| 512 | issue | CLOSED | Google Play Unsafe Encryption | CRYPTO-LEGACY | avoided by design |
| 513 | PR | MERGED | bug: fix build macos | APP-PKG | not applicable |
| 514 | issue | CLOSED | Still running on Flutter 3.7.0? | FLUTTER-WIRE | not applicable |
| 515 | issue | CLOSED | How to allow user to save/fill login info on/from Keychain/Keystore | NOA-MISC | no action |
| 516 | PR | MERGED | fix: crypto catch | CRYPTO-LEGACY-2 | avoided by design |
| 517 | PR | MERGED | build(deps): bump com.android.tools.build:gradle from 7.4.0 to 7.4.1 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 518 | PR | MERGED | build(deps): bump com.android.tools.build:gradle from 7.4.0 to 7.4.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 519 | issue | CLOSED | Read on iOS fails silently and returns null instead of throwing platform exception. | APP-STATUS | requirement |
| 520 | PR | MERGED | Update README.md | DEPS | not applicable |
| 521 | issue | CLOSED | read not working when Migrate a Windows project to set version information | WIN-SHARE | requirement |
| 522 | issue | CLOSED | Custom AES encryption key | KEY-HANDLING | avoided by design |
| 523 | issue | CLOSED | Path to secure storage for Windows Desktop | DOCS | requirement |
| 524 | issue | CLOSED | .read() return null since iOS 16.3.X even with the correct KeychainAccessibility | APP-QUERY | requirement |
| 525 | issue | CLOSED | Questions about IOSOptions accessibility - How to access enum | APP-OPTS | requirement |
| 526 | issue | CLOSED | Weak security with AES/CBC/PKCS7Padding | CRYPTO-LEGACY | avoided by design |
| 527 | issue | CLOSED | \[android\] Warning: Accessing hidden method Lsun/misc/Unsafe;-\>putObject | LINUX-BUILD | not applicable |
| 528 | issue | CLOSED | Flutter secure storage in Linux | NOA-MISC | no action |
| 529 | PR | MERGED | build(deps): bump androidx.security:security-crypto from 1.1.0-alpha04 to 1.1.0-alpha05 in /flutter\_secure\_sto | DEPS | not applicable |
| 530 | issue | CLOSED | Linux initial read stomps value with unlocking keychain | LINUX-KEYRING | requirement |
| 531 | issue | CLOSED | \[web\] getting null data after refresh app | WEB-CONTEXT | requirement |
| 532 | issue | CLOSED | Flutter secure storage not storing data after I closed app IOS | APP-QUERY | requirement |
| 533 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 7.4.1 to 7.4.2 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 534 | PR | MERGED | build(deps): bump com.android.tools.build:gradle from 7.4.1 to 7.4.2 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 535 | issue | CLOSED | How to store different different data types | FEATURE | no action |
| 536 | PR | MERGED | Perform Keychain operations asynchronously on a non-main thread. | BLOCK | requirement |
| 537 | PR | MERGED | Enhance security from AES-128 to AES-256 | CRYPTO-LEGACY | avoided by design |
| 538 | PR | MERGED | Add macOS App Groups Documentation | DOCS-2 | requirement |
| 539 | issue | CLOSED | Some kinds of keys cause app crash in Windows | KEY-CHARS | requirement |
| 540 | PR | MERGED | FFI based Windows implementation to solve key related issues | WIN-PKG | not applicable |
| 541 | issue | CLOSED | PlatformException (PlatformException(Exception encountered, read, javax.crypto.BadPaddingException: error:1e00 | CRYPTO-LEGACY | avoided by design |
| 542 | issue | CLOSED | PlatformException (PlatformException(Exception encountered, read, javax.crypto.BadPaddingException: error:1e00 | CRYPTO-LEGACY | avoided by design |
| 543 | issue | CLOSED | PlatformException (PlatformException(Exception encountered, read, javax.crypto.BadPaddingException: error:1e00 | CRYPTO-LEGACY | avoided by design |
| 544 | PR | MERGED | build(deps): bump subosito/flutter-action from 2.8.0 to 2.10.0 | DEPS | not applicable |
| 545 | issue | CLOSED | Linux libsecret error | LINUX-KEYRING | requirement |
| 546 | issue | CLOSED | How do you write a unit test for a Flutter/Dart class that uses FlutterSecureStorage? | FLUTTER-WIRE | not applicable |
| 547 | issue | CLOSED | \[iOS\] When you delete an app, the data is also deleted. It cannot be read back. | APP-BACKUP | requirement |
| 548 | issue | CLOSED | \[Android\]\[Question\] The plugin flutter\_secure\_storage requires your app to be migrated to the Android embeddin | CRYPTO-LEGACY | avoided by design |
| 549 | PR | MERGED | \[Linux\] Remove libjsoncpp1 dependency | LINUX-BUILD | not applicable |
| 550 | issue | CLOSED | Windows Secure Storage Details | DOCS | requirement |
| 551 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 7.4.1 to 8.0.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 552 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 7.4.2 to 8.0.0 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 553 | issue | CLOSED | \[Question\] Synchronization between devices | APP-SYNC | avoided by design |
| 554 | issue | CLOSED | \[Android\] PlatformException when invoking write value | CRASH-2 | requirement |
| 555 | issue | CLOSED | EXC\_BAD\_ACCESS when flutter\_secure\_storage is doing something | CRASH-2 | requirement |
| 556 | PR | MERGED | build(deps): bump androidx.security:security-crypto from 1.1.0-alpha05 to 1.1.0-alpha06 in /flutter\_secure\_sto | DEPS | not applicable |
| 557 | issue | CLOSED | Add compatibility with AGP 8 (Android Gradle Plugin) | AND-PKG | not applicable |
| 558 | issue | CLOSED | Error when opening an android project | AND-PKG | not applicable |
| 559 | issue | CLOSED | Android native plugin doesn't support AGP 8.0 | AND-PKG | not applicable |
| 560 | PR | MERGED | Fix compatibility with AGP 8.0 | NA-BUILD | not applicable |
| 561 | issue | CLOSED | Hardcoded encryption keys can jeopardize secure data storage and transmission within the mobile application by | KEY-HANDLING | avoided by design |
| 562 | issue | CLOSED | High attention point related to "Padding Oracle", reported by MobSF, when using encryption mode, "AES/CBC/PKCS | CRYPTO-LEGACY | avoided by design |
| 563 | issue | CLOSED | Delete fails on Flutter web | WEB-DELETE | requirement |
| 564 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 7.4.1 to 8.0.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 565 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 7.4.2 to 8.0.1 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 566 | issue | CLOSED | Android 13 - 1st read after opening the app returns null | ORDER | requirement |
| 567 | issue | CLOSED | Android Gradle is not building | AND-PKG | not applicable |
| 568 | issue | CLOSED | does the latest version uses | NOA-MISC | no action |
| 569 | issue | CLOSED | \[Android\] Bad Padding | CRYPTO-LEGACY | avoided by design |
| 570 | PR | MERGED | \[windows\] Error when calling delete() before application support path exists | MISSING | requirement |
| 571 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 7.4.1 to 8.0.2 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 572 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 7.4.2 to 8.0.2 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 573 | issue | CLOSED | \[bug\] \[macOS\]: Creates duplicate entries when write with a key is used | APP-UPSERT | requirement |
| 574 | issue | CLOSED | CompanyName and CompanyProduct on Windows are ignored when the lang-charset in the Runner.rc file is not 04090 | WIN-PKG | not applicable |
| 575 | PR | MERGED | \[Windows\] Fix charset problem | WIN-PKG | not applicable |
| 576 | issue | CLOSED | \[Android\] Allow use of userAuthenticationParameters | AND-BIO-KEY | requirement |
| 577 | issue | CLOSED | when using a IP address without domain name, the following error occurs on WEB | WEB-CONTEXT | requirement |
| 578 | issue | CLOSED | Keychain Sharing not Working with FlutterSecureStorage Plugin and App Flavors on iOS | APP-QUERY | requirement |
| 579 | issue | CLOSED | \[Question\] For web, how is the unique encryption key generated and securely reused? | WEB-ASK | no action |
| 580 | PR | CLOSED | update js dep to 0.6.7 | WEB-PKG | not applicable |
| 581 | issue | CLOSED | Write on Mac doesn't overwrite previous value; Delete works fine; | APP-UPSERT | requirement |
| 582 | PR | MERGED | Enable buildFeatures to support AGP8 | AND-PKG | not applicable |
| 583 | issue | CLOSED | iOS app crash but only in production | CRASH-MISC | requirement |
| 584 | issue | CLOSED | PKCS5 and PKCS7 are known to be vulnerable to oracle padding attacks | CRYPTO-LEGACY | avoided by design |
| 585 | issue | CLOSED | iOS app crash while read | APP-LOCKED | requirement |
| 586 | issue | CLOSED | \[bug\] \[macOS\]: Asks for confidential information stored in keychain multiple times even after pressing "Always | IOS-QUERY-2 | requirement |
| 587 | PR | MERGED | \[Android\] Get resetOnError from the options | AND-RESET | requirement |
| 588 | issue | CLOSED | How to access storage (read/write) in native part (Kotlin/Swift) ? | FLUTTER-WIRE | not applicable |
| 589 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 7.4.2 to 8.1.0 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 590 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 7.4.1 to 8.1.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 591 | issue | CLOSED | can't disable data restore from icloud / keychain on iOS | APP-BACKUP | requirement |
| 592 | issue | CLOSED | Calling READ during WRITE leds to NULL READ - Race condition, concurrency issue | ORDER | requirement |
| 593 | issue | CLOSED | \[iOS\] Map correctly the Keychain Attributes for Passwords | APP-OPTS | requirement |
| 594 | issue | CLOSED | Gradle build failed | AND-PKG | not applicable |
| 595 | issue | CLOSED | Keychain data on iOS gets deleted after some time | AND-LOSS | requirement |
| 596 | issue | CLOSED | Doesnt work on windows | NOA-MISC | no action |
| 597 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 7.4.2 to 8.1.1 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 598 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 7.4.1 to 8.1.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 599 | issue | CLOSED | Latest update on flutter\_secure\_storage\_windows 2.1.0 Broke app build | WIN-PKG | not applicable |
| 600 | issue | CLOSED | build porblem on macos sonoma xcode 15 | APP-PKG | not applicable |
| 601 | issue | CLOSED | \[Mobile\] Getting Flutter\_secure\_storage\_windows Error | WIN-PKG | not applicable |
| 602 | PR | MERGED | iOS Keychain - Use accessiblity option for all operations and throw PlatformException for SecErrors | APP-QUERY | requirement |
| 603 | issue | CLOSED | Windows Error | WIN-PKG | not applicable |
| 604 | issue | CLOSED | dependencies version | NA-BUILD | not applicable |
| 605 | issue | CLOSED | Reading same entries from different apps in Windows | WIN-SHARE | requirement |
| 606 | PR | CLOSED | Feature/handle os status | APP-ENT-2 | requirement |
| 607 | issue | CLOSED | The delete all function doesn't work with encryption. | WEB-DELETE | requirement |
| 608 | issue | CLOSED | Not working with Mockito | FLUTTER-WIRE | not applicable |
| 609 | PR | CLOSED | Fix IOS keychain critical bug | NOA-MISC | no action |
| 610 | PR | MERGED | build(deps): bump actions/checkout from 3 to 4 | DEPS | not applicable |
| 611 | issue | CLOSED | feature: support isolate | FEATURE | no action |
| 612 | issue | CLOSED | Can we also read the Certificates under Keychain Access using this library? If yes, how? | NOA-MISC | no action |
| 613 | issue | CLOSED | About encryption on iOS | DOCS | requirement |
| 614 | issue | CLOSED | Weird issue where for iOS only | NOA-MISC | no action |
| 615 | issue | CLOSED | Feature request: Add Biometric Authentication Process as an Option | AND-BIO-KEY | requirement |
| 616 | issue | CLOSED | App fails with: nlohmann::json\_abi\_v3\_11\_2::detail::type\_error | LINUX-BUILD | not applicable |
| 617 | PR | CLOSED | Linux fix: Search with schemas fails in cold keyrings | LINUX-2 | requirement |
| 618 | issue | CLOSED | Linux: Search with schemas fails in cold keyrings | LINUX-KEYRING | requirement |
| 619 | issue | CLOSED | \[web\] deleteAll clears entire local storage | WEB-DELETE | requirement |
| 620 | issue | CLOSED | iOS setup | APP-PKG | not applicable |
| 621 | PR | MERGED | Linux fix: Search with schemas fails in cold keyrings | LINUX-2 | requirement |
| 622 | issue | CLOSED | Flutter Secure Storage Data is lost sometimes in both Android and IOS, what should I do not to lost automatica | AND-LOSS | requirement |
| 623 | PR | MERGED | Fix native Android stacktraces in PlatformExceptions | LOG | requirement |
| 624 | PR | MERGED | build(deps): bump com.android.tools.build:gradle from 8.1.1 to 8.1.2 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 625 | PR | MERGED | build(deps): bump com.android.tools.build:gradle from 8.1.1 to 8.1.2 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 626 | PR | MERGED | ci: upgrade java for test | DEPS | not applicable |
| 627 | issue | CLOSED | Does changing storageCipherAlgorithm for androidOptions make previously written values update its algorithm? | CRYPTO-LEGACY | avoided by design |
| 628 | PR | MERGED | macOS Keychain - Use accessiblity option for all operations and throw PlatformException for SecErrors | APP-QUERY | requirement |
| 629 | PR | MERGED | Feature - Check availability of protected data on iOS and macOS | LOCK-2 | requirement |
| 630 | issue | CLOSED | \[Web\] Cannot read value of JWT token | WEB-CONTEXT | requirement |
| 631 | issue | CLOSED | fatal error C1083: Cannot open include file: 'include/flutter\_secure\_storage\_windows/flutter\_secure\_storage\_wi | WIN-PKG | not applicable |
| 632 | issue | CLOSED | StorageCipher initialization failed | AND-READ | requirement |
| 633 | issue | CLOSED | \[Web\] TypeError: undefined is not an object (evaluating 'dart.global.crypto.subtle.generateKey') | WEB-CONTEXT | requirement |
| 634 | issue | CLOSED | PathAccessException: The process cannot access the file because it is being used by another process (Windows o | WIN-FILE | requirement |
| 635 | issue | CLOSED | Change storage path of secure storage | NOA-MISC | no action |
| 636 | PR | MERGED | Adressing MacOS build warnings | APP-PKG | not applicable |
| 637 | PR | CLOSED | build(deps): bump subosito/flutter-action from 2.10.0 to 2.11.0 | DEPS | not applicable |
| 638 | PR | MERGED | Update README.md | DEPS | not applicable |
| 639 | issue | CLOSED | A naming issue in flutter\_secure\_storage\_platform\_interface | NA-BUILD | not applicable |
| 640 | PR | MERGED | Listener functionality | FEATURE | no action |
| 641 | PR | MERGED | Implement isCupertinoProtectedDataAvailable and onCupertinoProtectedDataAvailabilityChanged for web platform | FEATURE-2 | no action |
| 642 | issue | CLOSED | Do more people see an increase in issues on Samsung Galaxy S21 5G devices? | NOA-MISC | no action |
| 643 | issue | CLOSED | \[Linux\] read returns null when keyring is unlocked for the first time after boot | LINUX-KEYRING | requirement |
| 645 | PR | MERGED | build(deps): bump subosito/flutter-action from 2.10.0 to 2.12.0 | DEPS | not applicable |
| 646 | issue | CLOSED | IOSOptions lacks more context/documentation | APP-OPTS | requirement |
| 647 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 8.1.2 to 8.1.3 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 648 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 8.1.2 to 8.1.3 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 649 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 8.1.2 to 8.1.4 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 650 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 8.1.2 to 8.1.4 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 651 | issue | CLOSED | EncryptedSharedPreferences initialization failed - anomaly | CRYPTO-LEGACY | avoided by design |
| 652 | issue | CLOSED | Flutter+Android flavoring compatibility seems to be broken | AND-PKG | not applicable |
| 653 | issue | CLOSED | \[Android\] Storage does not survive device restart | AND-LOSS | requirement |
| 654 | PR | MERGED | Fix exposure of FSS API | AND-EMB | not applicable |
| 655 | issue | CLOSED | Enforce key stored in hardware | FEATURE | no action |
| 656 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 8.1.2 to 8.2.0 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 657 | PR | CLOSED | build(deps): bump actions/setup-java from 3 to 4 | DEPS | not applicable |
| 658 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 8.1.2 to 8.2.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 659 | issue | CLOSED | values are being shared between apps on ios. | IOS-QUERY-2 | requirement |
| 660 | issue | CLOSED | iOS: stored values are persisted even after app was removed | APP-BACKUP | requirement |
| 661 | issue | CLOSED | Running xcodebuild produces fatal error: module 'flutter\_secure\_sotrage' not found | APP-PKG | not applicable |
| 662 | PR | MERGED | Resolve error when no item exist (iOS) | APP-STATUS | requirement |
| 663 | issue | CLOSED | Cmake Error: a required package was not found | LINUX-BUILD | not applicable |
| 664 | PR | MERGED | Fix Android read data after boot | AND-LOSS | requirement |
| 665 | issue | CLOSED | Can you add support for 0.0.0.0 IP? | WEB-CONTEXT | requirement |
| 666 | issue | CLOSED | No such module 'flutter\_secure\_storage\_macos' | APP-PKG | not applicable |
| 667 | issue | CLOSED | Sync storage on OneDrive on Windows | DOCS | requirement |
| 668 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 8.1.2 to 8.2.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 669 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 8.1.2 to 8.2.1 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 670 | issue | CLOSED | Upgrade dependency js | WEB-PKG | not applicable |
| 671 | issue | CLOSED | How can you delete flutter secure storage data from an iphone? | APP-BACKUP | requirement |
| 672 | issue | CLOSED | Asymmetric cipher with insecure padding used | CRYPTO-LEGACY | avoided by design |
| 673 | PR | MERGED | \[LINUX\] Fix erase was called on null #616 | NATIVE-MEMORY | avoided by design |
| 674 | issue | CLOSED | Can run app with flutter\_secure\_storage | NOA-MISC | no action |
| 675 | issue | CLOSED | \[WEB\] Alternative to the experimental implementation using WebCrypto | WEB-ASK | no action |
| 676 | issue | CLOSED | Linux App Won't build | LINUX-BUILD | not applicable |
| 677 | issue | CLOSED | secure storage is deleted on app upgrade. | LOSS-2 | requirement |
| 678 | PR | CLOSED | \[windows\] Fix compilation error for flutter\_secure\_storage\_windows\_ffi.dart | WIN-PKG | not applicable |
| 679 | issue | CLOSED | Migrate to package:web | WEB-PKG | not applicable |
| 680 | PR | MERGED | Update js dependency and migrate to web | WEB-PKG | not applicable |
| 681 | PR | MERGED | Add melos to project | DEPS | not applicable |
| 682 | issue | CLOSED | When is new version coming with isCupertinoProtectedDataAvailable() etc | FEATURE-2 | no action |
| 683 | issue | CLOSED | Update flutter\_secure\_storage\_web depends on js ^0.6.3 | WEB-PKG | not applicable |
| 684 | issue | CLOSED | During archive process for ios the following exception occurs | APP-PKG | not applicable |
| 685 | issue | CLOSED | \[iOS\] Are there any plans to add Privacy Manifests? | APP-PKG | not applicable |
| 686 | issue | CLOSED | \[macOS\] Read always return null with signed and notarized app | APP-QUERY | requirement |
| 687 | issue | CLOSED | Android App Build Fails After Installing flutter\_secure\_storage | AND-PKG | not applicable |
| 688 | issue | CLOSED | \[Web\] Error: TypeError: 86400: type 'int' is not a subtype of type 'Iterable\<dynamic\>' | WEB-PKG | not applicable |
| 689 | issue | CLOSED | FlutterSecureStorage on macos build macOS (darwin) fails | APP-PKG | not applicable |
| 690 | issue | CLOSED | Flutter Secure Storage is Not working on Release Mode in Android 14 | NOA-MISC | no action |
| 691 | PR | CLOSED | chore: upgrade js from 0.6.3 to 0.7.1 | WEB-PKG | not applicable |
| 692 | issue | CLOSED | \[iOS\] Native app update | IOS-QUERY-2 | requirement |
| 693 | PR | CLOSED | updated js | WEB-PKG | not applicable |
| 694 | issue | CLOSED | App uses CBC with PKCS5/PKCS7 padding | CRYPTO-LEGACY | avoided by design |
| 695 | issue | CLOSED | \[WEB\] Upgrade of js Dependency in flutter\_secure\_storage\_web | WEB-PKG | not applicable |
| 696 | PR | MERGED | iOS - Add Privacy Manifest | APP-PKG | not applicable |
| 697 | issue | CLOSED | Implementing encryption with the same logic as FlutterSecureStorage | NOA-MISC | no action |
| 698 | PR | MERGED | Migrate dart:html and package:js to package:web and dart:js\_interop | WEB-PKG | not applicable |
| 699 | issue | CLOSED | Is this project still maintained? | NOA-MISC | no action |
| 700 | issue | CLOSED | I suspect a bug where my session keys are deleted and it causes my session to be logged out. | AND-LOSS | requirement |
| 701 | issue | CLOSED | Could not find gradle-8.1.1.jar (com.android.tools.build:gradle:8.1.1) | AND-PKG | not applicable |
| 702 | issue | CLOSED | Libsecret issues on fedora 39 | LINUX-KEYRING | requirement |
| 703 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 8.1.2 to 8.4.0 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 704 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 8.3.2 to 8.4.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 705 | PR | MERGED | build(deps): bump subosito/flutter-action from 2.12.0 to 2.16.0 | DEPS | not applicable |
| 706 | PR | MERGED | Update flutter\_drive.yml | DEPS | not applicable |
| 707 | issue | CLOSED | Error when updating the flutter\_secure\_storage\_platform\_interface 1.1.0 | NA-BUILD | not applicable |
| 708 | issue | CLOSED | Flutter secure storage 8.1.0 no longer works | FLUTTER-WIRE-2 | not applicable |
| 709 | issue | CLOSED | The specified item could not be found in the keychain | APP-STATUS | requirement |
| 710 | PR | MERGED | Return nil on iOS read if key is not found | APP-STATUS | requirement |
| 711 | issue | CLOSED | Message: The specified item already exists in the keychain, IOS | APP-UPSERT | requirement |
| 712 | PR | MERGED | Also set kSecUseDataProtectionKeychain on read for macos | APP-QUERY | requirement |
| 713 | issue | CLOSED | StorageCipher initialization failed cause by AndroidKeyStore not found | AND-LOAD | requirement |
| 714 | PR | CLOSED | Update master | DEPS | not applicable |
| 715 | PR | MERGED | feat: add onCupertinoProtectedDataAvailabilityChanged in non breaking way | FEATURE-2 | no action |
| 716 | issue | CLOSED | \[bug\] Race condition on write and read. | ORDER | requirement |
| 717 | PR | MERGED | Fix async race condition bug in storage operations | ORDER | requirement |
| 718 | PR | MERGED | bug: Macos return nil | APP-QUERY | requirement |
| 719 | PR | MERGED | fix: \[iOS / macOS\] pass accessibility to readAll and deleteAll | APP-QUERY | requirement |
| 720 | issue | CLOSED | deleteAll not working on iOS | APP-QUERY | requirement |
| 721 | PR | CLOSED | Fix: Update README.md | DEPS | not applicable |
| 722 | PR | MERGED | chore: fix/update example & integration CI | LAST-DEPS | not applicable |
| 723 | issue | CLOSED | A problem occurred configuring project ':flutter\_secure\_storage'. | AND-PKG | not applicable |
| 724 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 8.1.2 to 8.4.1 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 725 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 8.3.2 to 8.4.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 726 | issue | CLOSED | \[Web\] Encryption of saved value by app-specific key | WEB-ASK | no action |
| 727 | issue | CLOSED | PlatformException: Code: -25308. Help me! | APP-LOCKED | requirement |
| 728 | issue | CLOSED | requires js ^0.6.3 depends on js ^0.7.1 | WEB-PKG | not applicable |
| 729 | issue | CLOSED | Beware the androidx.security:security-crypto is deprecated | LAST-ESP | avoided by design |
| 730 | PR | MERGED | feat: Support for wrap and unwrap key on web platform | FEATURE-2 | no action |
| 731 | PR | MERGED | feat: Improve deleteAll in FlutterSecureStorageWeb | NAMES-R11-2 | requirement |
| 732 | issue | CLOSED | \[WEB\] Wasm support | FLUTTER-WIRE | not applicable |
| 733 | issue | CLOSED | onCupertinoProtectedDataAvailabilityChanged listener is not returning any value(true or false) even when isCup | FEATURE-2 | no action |
| 734 | PR | CLOSED | refactor: Cleanup optional values | NA-LINT | not applicable |
| 735 | issue | CLOSED | \> Failed to transform android.jar to match attributes {artifactType=android-platform-attr, org.gradle.librarye | AND-PKG | not applicable |
| 736 | PR | MERGED | Add useDataProtectionKeyChain parameter to macOS options | APP-QUERY | requirement |
| 737 | PR | CLOSED | Check contains key with readed value | CONTAINS | requirement |
| 738 | issue | CLOSED | InfoSec Documentation for Windows is missing | DOCS | requirement |
| 739 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 8.1.2 to 8.5.0 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 740 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 8.3.2 to 8.5.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 741 | PR | MERGED | Update README.md | DEPS | not applicable |
| 742 | issue | CLOSED | Question about files on Windows platform | DOCS | requirement |
| 743 | issue | CLOSED | Getting values in separate isolate fails - Code: -25308, Message: User interaction is not allowed | APP-LOCKED | requirement |
| 744 | issue | CLOSED | Reading a specific key or using read all returns a null from the store | ORDER | requirement |
| 745 | issue | CLOSED | Running on web fails when \_selectOptions are called | WEB-PKG | not applicable |
| 746 | issue | CLOSED | Recording with encrypted data | NOA-MISC | no action |
| 747 | issue | CLOSED | Linux read always empty | LINUX-KEYRING | requirement |
| 748 | issue | CLOSED | Cannot compile my project with AGP \>=8.0.0 due to R8 error | AND-PKG | not applicable |
| 749 | issue | CLOSED | I am trying to save data but for some reason it is not saved on Flutter web (Chrome) | WEB-CONTEXT | requirement |
| 750 | issue | CLOSED | app run crash on macos m1 pro | APP-ENT-2 | requirement |
| 751 | PR | MERGED | Fix for issue #711: The specified item already exists in the keychain, iOS Code -25299, -25300 | APP-UPSERT | requirement |
| 752 | issue | CLOSED | After reopen application, getting null values in IOS. But working in Android. | APP-QUERY | requirement |
| 753 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 8.1.2 to 8.5.1 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 754 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 8.3.2 to 8.5.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 755 | issue | CLOSED | flutter\_secure\_storage for web get error: type LegacyJavaScriptObject is not a subtype of FutureOr\<CryptoKey\> | WEB-PKG | not applicable |
| 756 | PR | MERGED | json.dump with indentations for linux | LINUX-BUILD | not applicable |
| 757 | issue | CLOSED | Error while reading data on windows | WIN-FILE | requirement |
| 758 | issue | CLOSED | SHA-1 is a weak hash known to have hash collisions. | CRYPTO-LEGACY | avoided by design |
| 759 | issue | CLOSED | App uses the encryption mode CBC with PKCS5/PKCS7 padding | CRYPTO-LEGACY | avoided by design |
| 760 | issue | CLOSED | After reopen application, getting null values in IOS. But working in Android | APP-QUERY | requirement |
| 761 | issue | CLOSED | Null check operator used on a null value | FLUTTER-WIRE | not applicable |
| 762 | issue | CLOSED | iOS: Breaking Change from 9.1.x forward, causing loss of data | APP-QUERY | requirement |
| 763 | PR | CLOSED | fix: allow null value for accessibility | APP-OPTS | requirement |
| 764 | PR | MERGED | Lower boundary of web dependency | WEB-PKG | not applicable |
| 765 | issue | CLOSED | Datas are not removed when app is uninstalled - I0S | APP-BACKUP | requirement |
| 766 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 8.3.2 to 8.5.2 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 767 | PR | CLOSED | build(deps): bump com.android.tools.build:gradle from 8.1.2 to 8.5.2 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 768 | PR | CLOSED | imp: remove deprecated storage function, upgrade to custom implementa… | CRYPTO-LEGACY-2 | avoided by design |
| 769 | issue | CLOSED | \[Android\] \[Beta\] Version 10 - Full migration to custom implementation of SecureSharedPreferences | LAST-ESP | avoided by design |
| 770 | PR | MERGED | doc: add docs for apple options | DOCS-2 | requirement |
| 771 | issue | CLOSED | pod player video not play | FLUTTER-WIRE | not applicable |
| 772 | issue | CLOSED | Unable to build the library in NixOS | FLUTTER-WIRE | not applicable |
| 773 | issue | CLOSED | \[BUG\] \[WEB\] unable to load from storage after F5 page refresh | WEB-CONTEXT | requirement |
| 774 | PR | CLOSED | chore(deps): bump org.jetbrains.kotlin.android from 1.9.24 to 2.0.10 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 775 | PR | CLOSED | chore(deps): bump com.android.application from 8.4.1 to 8.5.2 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 776 | PR | MERGED | Remove duplicates from CHANGELOG.md | DEPS | not applicable |
| 777 | PR | CLOSED | chore(deps): bump org.jetbrains.kotlin.android from 1.9.24 to 2.0.20 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 778 | issue | CLOSED | \[Linux\] Warning "libsecret\_error: Failed to unlock the keyring" | LINUX-KEYRING | requirement |
| 779 | issue | CLOSED | \[flutter\_secure\_storage\_windows\] Migrate to win32 5.5.1 or higher | WIN-PKG | not applicable |
| 780 | PR | MERGED | Migrate to win32 5.5.4 | WIN-PKG | not applicable |
| 781 | issue | CLOSED | build fails completely, upgrade js package to (js ^0.7.1) | WEB-PKG | not applicable |
| 782 | PR | CLOSED | Fix null check error and improve error handling in web implementation | WEB-READ | requirement |
| 783 | PR | CLOSED | chore(deps): bump com.android.application from 8.4.1 to 8.6.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 784 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.5.1 to 8.6.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 785 | issue | CLOSED | An error occurs on write when the item already exists, when it should just be replaced | APP-UPSERT | requirement |
| 786 | issue | CLOSED | \[BUG\] \[WEB\] Unsupported operation: Platform.\_operatingSystem error upon any operation | WEB-PKG | not applicable |
| 787 | PR | CLOSED | bump flutter version in web | WEB-PKG | not applicable |
| 788 | issue | CLOSED | \[WEB\]\[BUG\] MissingPluginException after performing any action | WEB-PKG | not applicable |
| 789 | issue | CLOSED | StorageCipher initialization failed on android 11 | AND-READ | requirement |
| 790 | issue | CLOSED | what do to in this | NOA-MISC | no action |
| 791 | PR | MERGED | \[iOS\] Use serial queue for execution of iOS keychain operations | BLOCK | requirement |
| 792 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.5.1 to 8.6.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 793 | PR | CLOSED | chore(deps): bump com.android.application from 8.4.1 to 8.6.1 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 794 | issue | CLOSED | Crash on startup after update to XCode 16 (iOS 18) | CRASH-MISC | requirement |
| 795 | issue | CLOSED | \[DOCS\]\[WINDOWS\] Difference between pub.dev documentation and project README | DOCS | requirement |
| 796 | PR | CLOSED | \[DOCS\]\[WINDOWS\] Difference between pub.dev documentation and project README | DEPS | not applicable |
| 797 | issue | CLOSED | \[Urgent\] Saved data automatically removed after 2,3 hours from Android and IOS devices. | AND-LOSS | requirement |
| 798 | issue | CLOSED | \[macOS\] macOS app does not launch after keychain sharing capability was selected | APP-STATUS | requirement |
| 799 | PR | CLOSED | fix gradle namespace | DEPS | not applicable |
| 800 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.5.1 to 8.7.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 801 | PR | CLOSED | chore(deps): bump com.android.application from 8.4.1 to 8.7.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 802 | issue | CLOSED | FATAL EXCEPTION: com.it\_nomads.fluttersecurestorage.worker | CRASH-2 | requirement |
| 803 | issue | CLOSED | Push new Release for Web Update | NOA-MISC | no action |
| 804 | issue | CLOSED | Secure storage crashes with: "A required entitlement isn't present" on MacOS | APP-STATUS | requirement |
| 805 | issue | CLOSED | FlutterError - ParallelWaitError: Instance of 'StorageException' on iOS | FLUTTER-WIRE | not applicable |
| 806 | issue | CLOSED | flutter\_scure\_storage does not work with android studio lady bug | NA-BUILD | not applicable |
| 807 | PR | CLOSED | chore(deps): bump org.jetbrains.kotlin.android from 1.9.24 to 2.0.21 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 808 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.5.1 to 8.7.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 809 | PR | CLOSED | chore(deps): bump com.android.application from 8.4.1 to 8.7.1 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 810 | issue | CLOSED | App getting stuck on splash screen due to encrypted shared preference intialization issue | CRYPTO-LEGACY | avoided by design |
| 811 | PR | MERGED | chore(deps): bump futureware-tech/simulator-action from 3 to 4 | DEPS | not applicable |
| 812 | issue | CLOSED | Support for WASM by moving from dart:html to package:web | WEB-PKG | not applicable |
| 813 | issue | CLOSED | Secure Storage retuning null after app restart on Windows | WIN-FILE | requirement |
| 814 | PR | CLOSED | Set minSdkVersion 23 and delete old encryption codes | CRYPTO-LEGACY-2 | avoided by design |
| 815 | PR | CLOSED | chore(deps): bump com.android.application from 8.4.1 to 8.7.2 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 816 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.5.1 to 8.7.2 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 817 | issue | CLOSED | Data stored on android device is not persist | AND-BACKUP | requirement |
| 818 | issue | CLOSED | Update js | WEB-PKG | not applicable |
| 819 | issue | CLOSED | flutter-pi support | WEB-PKG | not applicable |
| 820 | PR | CLOSED | Master | DEPS | not applicable |
| 821 | issue | CLOSED | Add Swift Package Manager support | APP-PKG | not applicable |
| 822 | PR | CLOSED | feat: Update default algorithms | CRYPTO-LEGACY-2 | avoided by design |
| 823 | issue | CLOSED | \[iOS\] Failed to build iOS app when add flutter\_secure\_storage | APP-PKG | not applicable |
| 824 | PR | MERGED | feat: Support sessionStorage option for web | FEATURE-2 | no action |
| 825 | issue | CLOSED | Storage is cleared when refreshing the web page on mobile | WEB-CONTEXT | requirement |
| 826 | issue | CLOSED | Potential Conflict: flutter\_secure\_storage (9.2.2) and flutter\_sound (9.17.0) | NA-BUILD | not applicable |
| 827 | PR | CLOSED | release of v9.2.2 | DEPS | not applicable |
| 828 | PR | CLOSED | feat: add JSON serialization to support storing various data types in FlutterSecureStorage | FEATURE | no action |
| 829 | issue | CLOSED | flutter\_secure\_storage\_linux/linux/flutter\_secure\_storage\_linux\_plugin.cc:193:50: error: use of undeclared ide | LINUX-BUILD | not applicable |
| 830 | issue | CLOSED | Migrate the Package to support AGP 8+ | AND-PKG | not applicable |
| 831 | PR | CLOSED | chore(deps): bump org.jetbrains.kotlin.android from 1.9.24 to 2.1.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 832 | issue | CLOSED | NullPointerException trying to call write() on some Android devices | AND-LOAD | requirement |
| 833 | PR | CLOSED | chore(deps): bump com.android.application from 8.4.1 to 8.7.3 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 834 | PR | MERGED | chore(deps): bump com.android.tools.build:gradle from 8.5.1 to 8.7.3 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 835 | issue | CLOSED | PlatformException(Unexpected security result code, Code: -25299, Message: The specified item already exists in | APP-UPSERT | requirement |
| 836 | issue | CLOSED | Missing SecAccessControlCreateWithFlags for iOS KeyChain query | APP-OPTS | requirement |
| 837 | issue | CLOSED | \[Android\]\[iOS\] Unable to read any data | IOS-QUERY-2 | requirement |
| 838 | PR | CLOSED | Merge pull request #1 from mogol/develop | NOA-MISC | no action |
| 839 | issue | CLOSED | Adding flutter\_secure\_storage to a new project causes build failures with:  Missing classes detected while run | AND-PKG | not applicable |
| 840 | issue | CLOSED | MissingPluginException(No implementation found for method read on channel plugins.it\_nomads.com/flutter\_secure | FLUTTER-WIRE | not applicable |
| 841 | issue | CLOSED | \[Question\] Can I ship encrypted keys in files with my app? | NOA-MISC | no action |
| 842 | PR | MERGED | Version 10 with migration | LEGACY-VERSION | avoided by design |
| 843 | issue | CLOSED | Strange dependency conflict | AND-PKG | not applicable |
| 844 | issue | CLOSED | when installing the package : version 4.2.1 installed | NA-BUILD | not applicable |
| 845 | PR | CLOSED | feat: update deps. fix #818 | LAST-DEPS | not applicable |
| 846 | PR | MERGED | release of v9.2.3 | DEPS | not applicable |
| 847 | PR | MERGED | Improvement/web | LAST-WEB | requirement |
| 848 | PR | MERGED | feat: release of v10.0.0-beta.1 | DEPS | not applicable |
| 849 | issue | CLOSED | Delete: Keys are not being deleted in iOS | IOS-QUERY-2 | requirement |
| 850 | issue | CLOSED | 10.0.0-beta.1 fails on release builds | FLUTTER-WIRE | not applicable |
| 851 | issue | CLOSED | FlutterSecureStorage plugin not available | FLUTTER-WIRE | not applicable |
| 852 | PR | MERGED | fix: force update tink library to fix release build issues | LAST-ESP | avoided by design |
| 853 | issue | CLOSED | v10.0.0-beta2: After Backup+Restore: Read and write fails without throwing exceptions | AND-BACKUP | requirement |
| 854 | PR | MERGED | fix: delete not working | IOS-QUERY-2 | requirement |
| 855 | issue | CLOSED | v10.0.0-beta.2 - MasterKey.java uses or overrides a deprecated API. | LAST-ESP | avoided by design |
| 856 | issue | CLOSED | secure storage web use deprecated js library cause conflict | WEB-PKG | not applicable |
| 857 | PR | MERGED | fix: update hotfix | NOA-MISC | no action |
| 858 | issue | CLOSED | v9.2.3 & 9.2.2 - issue while reading the data from Flutter Secure Storage | LEGACY-VERSION | avoided by design |
| 859 | PR | MERGED | Reintroduce deprecation warning supression. | NA-LINT | not applicable |
| 860 | PR | MERGED | feat: migrate ios and macos integration to darwin package | NA-BUILD | not applicable |
| 861 | PR | MERGED | hotfix: improve delete after failed write | AND-READ | requirement |
| 862 | issue | CLOSED | flutter\_secure\_storage: ^9.2.4 "FAILURE: Build failed with an exception. | AND-PKG | not applicable |
| 863 | PR | MERGED | chore(deps): bump com.android.application from 8.7.3 to 8.8.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 864 | PR | MERGED | chore(deps): bump com.android.tools.build:gradle from 8.7.3 to 8.8.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 865 | PR | MERGED | feat: add reset after failure, result.error after failure | AND-RESET | requirement |
| 866 | issue | CLOSED | v10.0.0-beta.4: Incorrect iOS parameter validation | APP-OPTS | requirement |
| 867 | PR | CLOSED | fix: remove dart:io to be compatible with wasm | WEB-PKG | not applicable |
| 868 | PR | MERGED | fix: remove unsupported query validation | IOS-QUERY-2 | requirement |
| 869 | issue | CLOSED | Flutter web exception: Uncaught Error: Null check operator used on a null value (getEncryptionKey) | WEB-READ | requirement |
| 870 | issue | CLOSED | Delete All Not working in Version 9.2.4 in iOS simulator 18.2 | FLUTTER-WIRE | not applicable |
| 871 | issue | CLOSED | PlatformException (PlatformException(Exception encountered, read, javax.crypto.BadPaddingException: error:1e00 | CRYPTO-LEGACY | avoided by design |
| 872 | issue | CLOSED | Potential Conflict: flutter\_secure\_storage and flutter\_sound | NA-BUILD | not applicable |
| 873 | PR | MERGED | feature: add access control parameter for apple | APP-OPTS | requirement |
| 874 | PR | CLOSED | chore(deps): bump org.jetbrains.kotlin.android from 1.9.25 to 2.1.10 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 875 | issue | CLOSED | Flutter secure storage not resetting the stored data once app is killed and reopened | AND-LOSS | requirement |
| 876 | issue | CLOSED | Data is not persisted on Android between uninstalls | AND-BACKUP | requirement |
| 877 | PR | MERGED | test: add unit test and code coverage | DEPS | not applicable |
| 878 | PR | CLOSED | feat: access control android | BIO-2 | requirement |
| 879 | PR | MERGED | test: add android caching, multi sdk testing | DEPS | not applicable |
| 880 | PR | MERGED | test: add unit tests for platform interface | DEPS | not applicable |
| 881 | PR | MERGED | test: update test to use included mock | DEPS | not applicable |
| 882 | PR | MERGED | test: add pod install to reduce build time | DEPS | not applicable |
| 883 | issue | CLOSED | Error on macOS (crash) | CRASH-2 | requirement |
| 884 | issue | CLOSED | Synchronizable working incorrectly | APP-SYNC | avoided by design |
| 885 | PR | MERGED | chore(deps): bump actions/cache from 3 to 4 | DEPS | not applicable |
| 886 | PR | MERGED | docs(Linux): remove mention of no longer needed libjsoncpp from README | DEPS | not applicable |
| 887 | issue | CLOSED | Data is lost after app reinstall on Android (even with allowBackup=true) | AND-BACKUP | requirement |
| 888 | issue | CLOSED | MSVC Compilation Error with /permissive- Flag | WIN-PKG | not applicable |
| 889 | issue | CLOSED | Android无法使用，编译报错 | NA-BUILD | not applicable |
| 890 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.8.0 to 8.8.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 891 | PR | CLOSED | chore(deps): bump com.android.application from 8.8.0 to 8.8.1 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 892 | issue | CLOSED | secure storage web causes conflict due to different js version | WEB-PKG | not applicable |
| 893 | issue | CLOSED | AndroidOptions does not recognise enctryptedSharedPreferences | CRYPTO-LEGACY | avoided by design |
| 894 | issue | CLOSED | Getting this secure storage issue while usinf bloc test and mockito | FLUTTER-WIRE | not applicable |
| 895 | issue | CLOSED | flutter pub add flutter\_secure\_storage installs 4.2.1 instead of 9.2.4 | FLUTTER-WIRE | not applicable |
| 896 | issue | CLOSED | Cannot find symbol import io.flutter.plugin.common.PluginRegistry.Registrar; | AND-PKG | not applicable |
| 897 | issue | CLOSED | Data is not removed when user uninstall the IOS app but its working fine in Android side. | APP-BACKUP | requirement |
| 898 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.8.0 to 8.8.2 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 899 | PR | CLOSED | chore(deps): bump com.android.application from 8.8.0 to 8.8.2 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 900 | issue | CLOSED | EncryptedSharedPreferences initialization failed | CRYPTO-LEGACY | avoided by design |
| 901 | issue | CLOSED | Crash on startup after update to XCode 16 .2(iOS 18.2) | CRASH-MISC | requirement |
| 903 | PR | CLOSED | Dependency report | DEPS | not applicable |
| 904 | PR | CLOSED | feat/update refs to develop | LAST-DEPS | not applicable |
| 905 | PR | MERGED | fix: remove warning on privacy manifest | APP-PKG | not applicable |
| 906 | PR | CLOSED | \[Android\] Optimize Secure Storage by Replacing Single Worker Thread with Thread Pool | ORDER-2 | requirement |
| 907 | PR | CLOSED | chore(deps): bump com.android.application from 8.8.0 to 8.9.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 908 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.8.0 to 8.9.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 909 | issue | CLOSED | Error writing to secure storage: Null check operator used on a null value" | FLUTTER-WIRE | not applicable |
| 910 | issue | CLOSED | \[ios\] EXC\_BAD\_ACCESS (code=1, address=0x0) | CRASH-MISC | requirement |
| 911 | PR | MERGED | chore(deps): bump com.google.crypto.tink:tink-android from 1.16.0 to 1.17.0 in /flutter\_secure\_storage/android | ESP-TINK | avoided by design |
| 912 | issue | CLOSED | Issue with readAll and deleteAll Methods After Upgrading to 10.0.0-beta.4 | AND-STORE-ID | requirement |
| 913 | PR | CLOSED | chore(deps): bump org.jetbrains.kotlin.android from 1.9.25 to 2.1.20 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 914 | issue | CLOSED | Both Android options preferencesKeyPrefix and sharedPreferencesName are ignored in V10.0-beta versions. | AND-STORE-ID | requirement |
| 915 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.8.0 to 8.9.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 916 | PR | CLOSED | chore(deps): bump com.android.application from 8.8.0 to 8.9.1 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 917 | issue | CLOSED | Saved data on iOS is not secure | DOCS | requirement |
| 918 | issue | CLOSED | Can not run app in iOS simulator 17.2 | FLUTTER-WIRE | not applicable |
| 919 | issue | CLOSED | Dependencies issue with flutter\_secure\_storage\_web | WEB-PKG | not applicable |
| 920 | issue | CLOSED | Linux deprecated literal operator | LINUX-BUILD | not applicable |
| 921 | PR | CLOSED | Fix macOS Privacy Manifest Integration to Prevent CFBundleExecutable Validation Error | APP-PKG | not applicable |
| 922 | issue | CLOSED | \[Windows\] Frequent Corruption of SecureStorage file | WIN-FILE | requirement |
| 923 | PR | MERGED | Update json.hpp | DEPS | not applicable |
| 924 | PR | CLOSED | chore(deps): bump com.android.application from 8.8.0 to 8.9.2 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 925 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.8.0 to 8.9.2 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 926 | PR | CLOSED | Create sonar-project.properties | DEPS | not applicable |
| 927 | issue | CLOSED | Typed flutter\_secure\_storage | FEATURE | no action |
| 928 | PR | MERGED | check if window has secureContext | WEB-CONTEXT | requirement |
| 929 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.8.0 to 8.10.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 930 | PR | CLOSED | chore(deps): bump com.android.application from 8.8.0 to 8.10.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 931 | PR | MERGED | docs(linux): replace ksecretsservice with kwalletmanager in README | LINUX-DOC | requirement |
| 932 | issue | CLOSED | Request for the WebAssembly version release | WEB-PKG | not applicable |
| 933 | issue | CLOSED | BadPaddingException when storing JWT in Flutter Secure Storage on Android | CRYPTO-LEGACY | avoided by design |
| 934 | PR | CLOSED | chore(deps): bump org.jetbrains.kotlin.android from 1.9.25 to 2.1.21 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 935 | issue | CLOSED | Any plan to add support for AES-GCM mode instead of CBC in this package? | CRYPTO-LEGACY | avoided by design |
| 936 | PR | MERGED | chore: update minSdkVersion from 23 to 24 | NA-BUILD | not applicable |
| 937 | issue | CLOSED | Flutter app crashes on launch with current develop branch | CRASH-2 | requirement |
| 938 | issue | CLOSED | encryption mode CBC with PKCS5/PKCS7 padding | CRYPTO-LEGACY | avoided by design |
| 939 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.8.0 to 8.10.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 940 | PR | CLOSED | chore(deps): bump com.android.application from 8.8.0 to 8.10.1 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 941 | issue | CLOSED | "Data has been reset" returned from read() | AND-RESET | requirement |
| 942 | issue | CLOSED | \[Web\] Experimental to Stable Next Steps | NOA-MISC | no action |
| 943 | issue | CLOSED | \[Windows\] Sharing of key-value pair via flutter\_secure\_storage package | WIN-SHARE | requirement |
| 944 | issue | CLOSED | Why aesPreferencesKey is hardcoded? | KEY-HANDLING | avoided by design |
| 945 | issue | CLOSED | Android L support | ANDROID-MINAPI | avoided by design |
| 946 | PR | CLOSED | Delete on failure when sharedprefs fails to initialise | AND-READ | requirement |
| 947 | issue | CLOSED | FlutterSecureKeyStorage.xml is visible on rooted Android devices even when using encryptedSharedPreferences: t | KEY-HANDLING | avoided by design |
| 948 | PR | CLOSED | chore(deps): bump com.google.crypto.tink:tink-android from 1.17.0 to 1.18.0 in /flutter\_secure\_storage/android | ESP-TINK | avoided by design |
| 949 | issue | CLOSED | using groupId param shuts the app | APP-QUERY | requirement |
| 951 | issue | CLOSED | \[Linux\] Breaks Gnome Keyring with Empty Password | LINUX-KEYRING | requirement |
| 953 | issue | CLOSED | app crash | CRASH-MISC | requirement |
| 954 | PR | CLOSED | chore(deps): bump com.android.application from 8.8.0 to 8.11.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 955 | PR | CLOSED | chore(deps): bump org.jetbrains.kotlin.android from 1.9.25 to 2.2.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 956 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.8.0 to 8.11.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 957 | issue | CLOSED | Plugin error during compilation Flutter Linux Fedora | LINUX-BUILD | not applicable |
| 958 | issue | CLOSED | Error during compilation on Fedora | LINUX-BUILD | not applicable |
| 959 | issue | CLOSED | \[BUG\] Data not stored properly in flutter\_secure\_storage: ^10.0.0-beta.4 on some devices | KEYSTORE-READ-2 | requirement |
| 960 | issue | CLOSED | Upgrading iOS to use KeychainAccessibility.first\_unlock\_this\_device | APP-OPTS | requirement |
| 961 | issue | CLOSED | remove dart:html to be compatible with wasm | WEB-PKG | not applicable |
| 962 | PR | CLOSED | Refactor Android storage cipher implementations | CRYPTO-LEGACY-2 | avoided by design |
| 963 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.8.0 to 8.11.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 964 | PR | CLOSED | chore(deps): bump com.android.application from 8.8.0 to 8.11.1 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 965 | issue | CLOSED | json.hpp - error: identifier '\_json' preceded by whitespace in a literal operator declaration is deprecated - | LINUX-BUILD | not applicable |
| 966 | issue | CLOSED | \[Android\] Exception encountered, write | CRASH-2 | requirement |
| 967 | issue | CLOSED | writeAll Atomic Transaction | FEATURE | no action |
| 969 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.8.0 to 8.12.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 970 | PR | CLOSED | chore(deps): bump com.android.application from 8.8.0 to 8.12.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 971 | issue | CLOSED | 安卓卸载应用存储的信息会一并删除，ios不行，当app卸载后，重新安装依旧可以找到存储在钥匙串里的信息，请问怎么可以解决ios端的问题 | APP-BACKUP | requirement |
| 972 | issue | CLOSED | Users reporting Secure Storage contents deleted after updating to encryptedSharedPreferences: true on 9.2.4 | CRYPTO-LEGACY | avoided by design |
| 973 | PR | CLOSED | chore(deps): bump org.jetbrains.kotlin.android from 1.9.25 to 2.2.10 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 974 | PR | MERGED | chore(deps): bump actions/checkout from 4 to 5 | DEPS | not applicable |
| 975 | PR | CLOSED | Fix: Handle Keystore exceptions on iMin devices | KEYSTORE-READ-2 | requirement |
| 976 | PR | MERGED | chore(deps): bump actions/setup-java from 4 to 5 | DEPS | not applicable |
| 977 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.8.0 to 8.12.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 978 | PR | CLOSED | chore(deps): bump com.android.application from 8.8.0 to 8.12.1 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 979 | PR | MERGED | Explain the underscore being added automatically to the prefix | NAMES-R11-2 | requirement |
| 980 | issue | CLOSED | In Windows, I can create an app and read another apps secure storage. | WIN-SHARE | requirement |
| 981 | issue | CLOSED | \[Windows\] Debug build fails with LNK1104: cannot open flutter\_secure\_storage\_windows\_plugin.dll (Release works | WIN-PKG | not applicable |
| 982 | issue | CLOSED | Two instances of FlutterSecureStorage | AND-SERIAL | requirement |
| 983 | PR | CLOSED | chore(deps): bump com.android.application from 8.8.0 to 8.13.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 984 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.8.0 to 8.13.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 985 | PR | MERGED | chore(deps): bump actions/stale from 9 to 10 | DEPS | not applicable |
| 986 | issue | CLOSED | flutter\_secure\_storage\_web depends on js 0.6.7 | WEB-PKG | not applicable |
| 987 | PR | CLOSED | chore(deps): bump org.jetbrains.kotlin.android from 1.9.25 to 2.2.20 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 988 | issue | CLOSED | Support 16 KB Page Size as per Android New Policy | PAGE-16K | requirement |
| 989 | PR | MERGED | Add Secure Enclave Support to iOS/macOS | FEATURE-2 | no action |
| 990 | issue | CLOSED | flutter build web does not work if flutter\_secure\_storage is included | FLUTTER-WIRE | not applicable |
| 991 | PR | MERGED | chore(deps): bump gradle/actions from 4 to 5 | DEPS | not applicable |
| 992 | issue | CLOSED | issue on Android 11 devices | CRYPTO-LEGACY-2 | avoided by design |
| 993 | issue | CLOSED | issue on device Android 11 | NOA-MISC | no action |
| 994 | PR | MERGED | chore(deps): bump actions/upload-artifact from 4 to 5 | DEPS | not applicable |
| 995 | PR | CLOSED | chore(deps): bump com.google.crypto.tink:tink-android from 1.17.0 to 1.19.0 in /flutter\_secure\_storage/android | ESP-TINK | avoided by design |
| 996 | PR | CLOSED | chore(deps): bump org.jetbrains.kotlin.android from 1.9.25 to 2.2.21 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 997 | PR | CLOSED | Add soup | NOA-MISC | no action |
| 998 | PR | CLOSED | Update soup | NOA-MISC | no action |
| 999 | issue | CLOSED | EncryptedSharedPreferenced deprecation on android | CRYPTO-LEGACY | avoided by design |
| 1000 | issue | CLOSED | MissingPluginException(No implementation found for method readAll on channel plugins.it\_nomads.co... | FLUTTER-WIRE | not applicable |
| 1001 | PR | MERGED | Use json dump without inserting newlines when saving to keyring on linux | LINUX-BUILD | not applicable |
| 1002 | PR | MERGED | feat: android rewrite containing secure ciphers, biometric support and migration tools | CRYPTO-LEGACY-2 | avoided by design |
| 1003 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.8.0 to 8.13.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1004 | PR | CLOSED | chore(deps): bump com.android.application from 8.8.0 to 8.13.1 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1005 | issue | CLOSED | Support for partial/header-only encryption, custom encrypted file storage, and caching behavior | FEATURE | no action |
| 1006 | PR | MERGED | ci: fix analyzer issues | DEPS | not applicable |
| 1007 | issue | CLOSED | Encryption mode CBC with PKCS5/PKCS7 padding is vulnerable to padding oracle attack | CRYPTO-LEGACY | avoided by design |
| 1008 | issue | CLOSED | Support Swift Package Manager (SPM) | APP-PKG | not applicable |
| 1009 | PR | MERGED | ci: revert checkout version | DEPS | not applicable |
| 1010 | PR | MERGED | imp: add linux options test | NOA-MISC | no action |
| 1011 | issue | CLOSED | Unexpected security result code, Code: -25299, Message: The specified item already exists in the keychain., -2 | APP-UPSERT | requirement |
| 1012 | PR | MERGED | refactor keychain deletion logic for completeness (fix issue #1011) | APP-UPSERT | requirement |
| 1013 | issue | CLOSED | \[MacOS\] Failed to read data after upgrade from 10.0.0-beta.4 to 10.0.0-beta.5 | APP-QUERY | requirement |
| 1014 | issue | CLOSED | Clarity on Encrypted Shared Preferences | CRYPTO-LEGACY | avoided by design |
| 1015 | PR | CLOSED | Use shared LAContext to reuse biometrics for Secure Enclave ops | FEATURE-2 | no action |
| 1016 | issue | CLOSED | Unsupported operation: only works in secure contexts on Flutter 3.38.5 | WEB-CONTEXT | requirement |
| 1017 | issue | CLOSED | .read and .write calls completely block on linux | BLOCK | requirement |
| 1018 | PR | MERGED | chore(deps): bump com.google.crypto.tink:tink-android from 1.19.0 to 1.20.0 in /flutter\_secure\_storage/android | ESP-TINK | avoided by design |
| 1019 | PR | MERGED | chore(deps): bump com.android.tools.build:gradle from 8.13.1 to 8.13.2 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1020 | PR | MERGED | chore(deps): bump actions/cache from 4 to 5 | DEPS | not applicable |
| 1021 | PR | MERGED | chore(deps): bump com.android.application from 8.13.1 to 8.13.2 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1022 | PR | MERGED | chore(deps): bump actions/upload-artifact from 5 to 6 | DEPS | not applicable |
| 1023 | issue | CLOSED | \[Android\] Multiple instances with different sharedPreferencesName share cached preferences, causing cross-name | AND-STORE-ID | requirement |
| 1024 | PR | MERGED | chore(deps): bump org.jetbrains.kotlin.android from 2.2.21 to 2.3.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1025 | issue | CLOSED | App uses the encryption mode CBC with PKCS5/PKCS7 padding // Flutter secure storage 10.0.0version | CRYPTO-LEGACY | avoided by design |
| 1026 | issue | CLOSED | secureStorageDeleteAll() function causes app crash | AND-STORE-ID | requirement |
| 1027 | issue | CLOSED | android\_options: resetOnError discrepancy | AND-RESET | requirement |
| 1028 | PR | MERGED | docs(AndroidOptions): clarify resetOnError default value and fix typo | AND-RESET | requirement |
| 1029 | issue | CLOSED | Can't get biometrics to work on iOS simulator in example app | FLUTTER-WIRE | not applicable |
| 1030 | issue | CLOSED | Does changing the KeychainAccessibility require migration | APP-QUERY | requirement |
| 1031 | PR | CLOSED | \[WIP\] fix(android): isolate storage instances per shared preferences name (fix: #1023) | AND-STORE-ID | requirement |
| 1032 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.13.2 to 9.0.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1033 | PR | CLOSED | chore(deps): bump com.android.application from 8.13.2 to 9.0.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1035 | issue | CLOSED | Encryption mode CBC with PKCS5/PKCS7 padding | CRYPTO-LEGACY | avoided by design |
| 1036 | PR | MERGED | Android: Restore native access to FlutterSecureStorage | AND-EMB | not applicable |
| 1037 | issue | CLOSED | Package sets minSdkVersion to 24 instead of 23 | AND-PKG | not applicable |
| 1038 | PR | MERGED | fix: api to 23 again | AND-PKG | not applicable |
| 1039 | PR | CLOSED | chore(deps): bump gradle-wrapper from 7.4.2 to 9.3.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1040 | PR | CLOSED | chore(deps): bump gradle-wrapper from 8.13 to 9.3.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1041 | PR | CLOSED | Fix #1038: Android minSdkVersion should be 23, not 24 | AND-PKG | not applicable |
| 1042 | issue | CLOSED | \[Bug\]\[Android\] Andriod.Biometrics retry causes crash | AND-BIO-PROMPT | requirement |
| 1043 | issue | CLOSED | Migration Failures on Android when updating | CRYPTO-LEGACY | avoided by design |
| 1044 | PR | CLOSED | chore(deps): bump gradle-wrapper from 7.4.2 to 9.3.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1045 | PR | CLOSED | chore(deps): bump gradle-wrapper from 8.13 to 9.3.1 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1046 | issue | CLOSED | PathAccessException on restart | WIN-FILE | requirement |
| 1047 | PR | MERGED | fix: align macOS options keys in darwin plugin | AUTH-NAMES | avoided by design |
| 1048 | PR | MERGED | chore(deps): bump org.jetbrains.kotlin.android from 2.3.0 to 2.3.10 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1049 | PR | MERGED | fix: pin very\_good\_analysis for breaking changes | DEPS | not applicable |
| 1050 | issue | CLOSED | web write take too much time | LARGE | requirement |
| 1051 | PR | MERGED | fix: continue on android isolate storage instances per shared preferences names by making key unique | AND-STORE-ID | requirement |
| 1052 | PR | MERGED | imp: update test for apple | NOA-MISC | no action |
| 1053 | issue | CLOSED | issue at documantion declartion | NA-LINT | not applicable |
| 1054 | PR | CLOSED | chore(deps): bump com.android.application from 8.13.2 to 9.0.1 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1055 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.13.2 to 9.0.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1056 | issue | CLOSED | Android ignores biometricPromptTitle and biometricPromptSubtitle in v10.0.0 (key mismatch) | AUTH-NAMES | avoided by design |
| 1057 | PR | MERGED | fix(android): align biometric prompt option keys | AUTH-NAMES | avoided by design |
| 1058 | issue | CLOSED | \[ios\] Adopt UISceneDelegate for iOS 27 | APP-PKG | not applicable |
| 1059 | issue | CLOSED | \[Android\]\[v10.0.0\] Crash: IllegalStateException: Reply already submitted after failed biometric retry "Android | AND-BIO-PROMPT | requirement |
| 1060 | PR | CLOSED | fix(android): prevent duplicate biometric result callbacks | AND-BIO-PROMPT | requirement |
| 1061 | PR | CLOSED | fix(web): defensive localStorage iteration in readAll() to ignore/remove corrupted entries | WEB-READ | requirement |
| 1062 | PR | CLOSED | fix(web): defensive localStorage iteration in readAll() | WEB-READ | requirement |
| 1063 | issue | CLOSED | fix(web): defensive readAll() to avoid Storage.forEach crash (edge-case corrupted localStorage) | WEB-READ | requirement |
| 1064 | issue | CLOSED | KeyStore exception on first run | AND-READ | requirement |
| 1065 | PR | CLOSED | Codex/biometric error codes work lib | BIO-CODES-2 | requirement |
| 1066 | PR | MERGED | chore(deps): bump actions/upload-artifact from 6 to 7 | DEPS | not applicable |
| 1067 | PR | MERGED | chore(deps): bump futureware-tech/simulator-action from 4 to 5 | DEPS | not applicable |
| 1068 | issue | CLOSED | \[Android\] StorageCipher initialization failed after upgrading to v10.x - Certificate not found in KeyStore | AND-READ | requirement |
| 1069 | PR | MERGED | Add crash-resistant migration with backup protection | CRYPTO-LEGACY | avoided by design |
| 1070 | PR | CLOSED | chore(deps): bump com.android.application from 8.13.2 to 9.1.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1071 | PR | CLOSED | chore(deps): bump gradle-wrapper from 7.4.2 to 9.4.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1072 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.13.2 to 9.1.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1073 | PR | CLOSED | chore(deps): bump gradle-wrapper from 8.13 to 9.4.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1074 | issue | CLOSED | Documentation update for iOS Secure storage - key lookup returns null after hot restart on iOS device | APP-BACKUP | requirement |
| 1075 | issue | CLOSED | PlatformException - Attempt to invoke interface method 'byte\[\] com.it\_nomads.fluttersecurestorage.ciphers.f.d( | AND-LOAD | requirement |
| 1076 | PR | CLOSED | ## fix(iOS): kSecAttrSynchronizable silently dropped when no access control flags are set | APP-SYNC | avoided by design |
| 1077 | PR | MERGED | chore(deps): bump gradle-wrapper from 7.4.2 to 9.4.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1078 | PR | CLOSED | chore(deps): bump gradle-wrapper from 8.13 to 9.4.1 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1079 | issue | CLOSED | \[Android\] v10 Migrator fails to decrypt v9 data saved with AES\_GCM\_NoPadding | CRYPTO-LEGACY | avoided by design |
| 1080 | PR | MERGED | fix(android): guard MethodRunner payload to prevent NPE crash | AND-LOAD | requirement |
| 1081 | PR | MERGED | chore(deps): bump codecov/codecov-action from 5 to 6 | DEPS | not applicable |
| 1082 | PR | MERGED | chore(deps): bump com.google.crypto.tink:tink-android from 1.20.0 to 1.21.0 in /flutter\_secure\_storage/android | ESP-TINK | avoided by design |
| 1083 | PR | MERGED | fix: crash on biometric failure | AND-BIO-PROMPT | requirement |
| 1084 | issue | CLOSED | NullPointerException in FlutterSecureStorage.initialize() when Activity is paused - Android 13 | AND-LOAD | requirement |
| 1085 | PR | MERGED | Fix compile on VS2022 | WIN-PKG | not applicable |
| 1086 | PR | MERGED | feat: native tests | NOA-MISC | no action |
| 1087 | PR | MERGED | fix: potential key skipping in readAll | ORDER | requirement |
| 1088 | PR | MERGED | feat: tests for web | NOA-MISC | no action |
| 1089 | PR | MERGED | chore(deps): bump actions/upload-artifact from 6 to 7 | DEPS | not applicable |
| 1090 | PR | CLOSED | chore(deps): bump gradle-wrapper from 8.13 to 9.4.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1091 | PR | MERGED | chore(deps): bump org.robolectric:robolectric from 4.13 to 4.16.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1092 | PR | MERGED | chore(deps): bump org.jetbrains.kotlin.android from 2.3.10 to 2.3.20 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 1093 | PR | MERGED | feat: native tests for windows | WIN-PKG | not applicable |
| 1094 | issue | CLOSED | java.lang.NullPointerException: Attempt to invoke virtual method 'void O9.l.A(O9.p, O9.u)' on a null object re | AND-LOAD | requirement |
| 1095 | issue | CLOSED | Need to update dependency win32 to version 6.0.0 | WIN-PKG | not applicable |
| 1096 | PR | CLOSED | chore: update version numbers and dependencies for flutter\_secure\_sto… | LAST-DEPS | not applicable |
| 1097 | PR | MERGED | fix: align android default to the flutter one | AUTH-NAMES-2 | avoided by design |
| 1098 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.13.2 to 9.1.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1099 | PR | CLOSED | chore(deps): bump com.android.application from 8.13.2 to 9.1.1 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1100 | PR | MERGED | fix: kSecAttrSynchronizable silently dropped when no access control flags are set | APP-SYNC | avoided by design |
| 1101 | PR | MERGED | docs: add iOS Keychain hot restart troubleshooting guide (#1074) | DEPS | not applicable |
| 1102 | issue | CLOSED | pubspec error | AND-PKG | not applicable |
| 1103 | issue | CLOSED | This repository seems to be abandoned; the issues from three years ago remain unresolved. | NOA-MISC | no action |
| 1104 | issue | CLOSED | errSecMissingEntitlement (-34018) on macOS when app is signed with Developer ID (outside App Store) — even wit | APP-STATUS | requirement |
| 1105 | issue | CLOSED | Bump win32 to ^6.0.0 in flutter\_secure\_storage\_windows | WIN-PKG | not applicable |
| 1106 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.13.2 to 9.2.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1107 | PR | MERGED | chore(deps): bump org.jetbrains.kotlin.android from 2.3.20 to 2.3.21 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 1108 | PR | CLOSED | chore(deps): bump com.android.application from 8.13.2 to 9.2.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1109 | issue | CLOSED | Bug: \[Android\] v10 sharedPreferencesName/preferencesKeyPrefix silently ignored on subsequent FlutterSecureStor | AND-STORE-ID | requirement |
| 1110 | PR | CLOSED | fix(flutter\_secure\_storage\_windows): Support win32 6.x | WIN-PKG | not applicable |
| 1111 | PR | CLOSED | chore(deps): bump gradle-wrapper from 8.13 to 9.5.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1112 | PR | CLOSED | chore(deps): bump gradle-wrapper from 8.13 to 9.5.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1113 | issue | CLOSED | Update Android Gradle script to avoid Groovy space-assignment deprecations | AND-PKG | not applicable |
| 1114 | PR | MERGED | Fix Android Gradle space-assignment deprecations | DEPS | not applicable |
| 1115 | issue | CLOSED | Failed to generate key pair (StorageCipher initialization failed Caused by: android.security.KeyStoreException | AND-READ | requirement |
| 1116 | PR | MERGED | feat: linux native tests | NOA-MISC | no action |
| 1117 | PR | MERGED | fix: win32 upgrade & other improvements | WIN-PKG | not applicable |
| 1118 | PR | CLOSED | feat: migration tool for secure enclave and resetOnError function | CRYPTO-LEGACY | avoided by design |
| 1119 | issue | CLOSED | Bug: iOS Build issue with flutter\_secure\_storage\_darwin 0.3.0: 'AES' is only available in iOS 13.0 or newer | APP-PKG | not applicable |
| 1120 | PR | MERGED | Update Package.swift | APP-PKG | not applicable |
| 1121 | PR | MERGED | fix: ios 12 build | APP-PKG | not applicable |
| 1122 | PR | MERGED | fix: update DPAPI FFI calls for win32 6.0.0 compatibility | WIN-PKG | not applicable |
| 1123 | PR | CLOSED | Raise Darwin minimums for CryptoKit APIs | APP-PKG | not applicable |
| 1124 | issue | CLOSED | Bug: Concurrent writes not handled correctly | ORDER | requirement |
| 1125 | issue | CLOSED | \[Android\] Issues with enforceBiometrics | AND-BIO-PROMPT | requirement |
| 1126 | issue | CLOSED | storageNamespace deletes previous data from sharedPreferencesName | AND-STORE-ID | requirement |
| 1127 | PR | CLOSED | chore(deps): bump com.android.tools.build:gradle from 8.13.2 to 9.2.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1128 | PR | CLOSED | chore(deps): bump com.android.application from 8.13.2 to 9.2.1 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1129 | PR | MERGED | chore(deps): bump nick-fields/retry from 3 to 4 | DEPS | not applicable |
| 1130 | PR | MERGED | imp: deprecate old algorithms for v11 | CRYPTO-LEGACY | avoided by design |
| 1131 | PR | MERGED | fix: windows concurrent operations | ORDER | requirement |
| 1132 | PR | MERGED | fix: linux locked keyring | LINUX-KEYRING | requirement |
| 1133 | PR | MERGED | fix: lock on deleteAll and containsKey | ORDER | requirement |
| 1134 | PR | MERGED | fix: macos plugin registration | APP-PKG | not applicable |
| 1135 | PR | MERGED | feat: v11 remove deprecated members | CRYPTO-LEGACY | avoided by design |
| 1136 | PR | MERGED | fix: only add kSecUseDataProtectionKeychain if param is set to true | APP-QUERY | requirement |
| 1137 | issue | CLOSED | Platform exception on ios ? | IOS-QUERY-2 | requirement |
| 1138 | issue | CLOSED | Keystore key generation failed (on Android TV) | AND-READ | requirement |
| 1139 | PR | CLOSED | chore(deps): bump gradle-wrapper from 8.13 to 9.5.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1140 | PR | CLOSED | chore(deps): bump gradle-wrapper from 8.13 to 9.5.1 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1141 | PR | MERGED | docs(readme): improve Linux platform setup / build (libsecret) | LINUX-DOC | requirement |
| 1142 | issue | CLOSED | \[Android\] AGP 9 / built-in Kotlin compatibility | AND-PKG | not applicable |
| 1143 | PR | MERGED | feat: add AndroidBiometricType option to restrict authentication to strong biometrics only | AND-BIO-KEY | requirement |
| 1144 | issue | CLOSED | deleteAll deletes unrelated data | AND-STORE-ID | requirement |
| 1145 | issue | CLOSED | Version 10.2.0 crashes at launch with EXC\_BAD\_ACCESS. | CRASH-MISC | requirement |
| 1146 | PR | MERGED | fix: linux doc typo, add more details section | LINUX-DOC | requirement |
| 1147 | PR | MERGED | fix: reset stale cipher IV when biometric auth fails or is cancelled | AND-BIO-PROMPT | requirement |
| 1148 | PR | MERGED | fix: defer preferences assignment until cipher initialization succeeds | AND-READ | requirement |
| 1149 | issue | CLOSED | \[SwiftPM\] Package.swift declares iOS 12.0 minimum but uses CryptoKit APIs (iOS 13.0+), causing Swift compiler | APP-PKG | not applicable |
| 1150 | PR | MERGED | fix: raised minimum iOS deployment target from 12.0 to 13.0 to fix Sw… | APP-PKG | not applicable |
| 1151 | PR | MERGED | feat: agp 9 for example app | AND-PKG | not applicable |
| 1152 | PR | MERGED | chore(deps): bump actions/dependency-review-action from 4 to 5 | DEPS | not applicable |
| 1153 | PR | MERGED | chore(deps): bump nanasess/setup-chromedriver from 2 to 3 | DEPS | not applicable |
| 1154 | PR | CLOSED | chore(deps): bump very\_good\_analysis from 10.0.0 to 10.2.0 in /flutter\_secure\_storage | DEPS | not applicable |
| 1155 | PR | CLOSED | chore(deps): bump very\_good\_analysis from 7.0.0 to 10.2.0 in /flutter\_secure\_storage\_web | DEPS | not applicable |
| 1156 | PR | CLOSED | chore(deps): bump very\_good\_analysis from 7.0.0 to 10.2.0 in /flutter\_secure\_storage\_windows | DEPS | not applicable |
| 1157 | PR | CLOSED | chore(deps): bump very\_good\_analysis from 7.0.0 to 10.2.0 in /flutter\_secure\_storage\_platform\_interface | DEPS | not applicable |
| 1158 | issue | CLOSED | iOS loses access to stored values after update to flutter\_secure\_storage 10.1.0 | APP-QUERY | requirement |
| 1159 | issue | CLOSED | Add Swift Package Manager support | APP-PKG | not applicable |
| 1160 | PR | CLOSED | chore(deps): bump org.jetbrains.kotlin.android from 2.3.21 to 2.4.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1161 | issue | CLOSED | Android: strongBiometricOnly read/write hangs forever when the biometric prompt is cancelled | AND-BIO-PROMPT | requirement |
| 1162 | PR | MERGED | chore(deps): bump codecov/codecov-action from 6 to 7 | DEPS | not applicable |
| 1163 | PR | CLOSED | Fixed KeyringLocked error on Linux. Improved error handling on Linux. | LINUX-KEYRING | requirement |
| 1164 | issue | CLOSED | iOS: kSecAttrAccessible used as strict search filter in 0.3.x breaks reads for items with different accessibil | APP-QUERY | requirement |
| 1165 | issue | CLOSED | \[Android\] AEADBadTagException on biometric read after app restart — StorageCipherImplementationAES23 cipher ke | AND-READ | requirement |
| 1166 | issue | CLOSED | Migration from encryptedSharedPreferences=false to new dependency versions fail and permanently deletes data ( | CRYPTO-LEGACY | avoided by design |
| 1167 | PR | MERGED | feat: add requireBiometricConfirmation option to AndroidOptions | AND-BIO-KEY | requirement |
| 1168 | PR | MERGED | fix: remove redundant ./ prefix from part directives | NA-LINT | not applicable |
| 1169 | PR | MERGED | chore(deps): bump actions/cache from 5 to 6 | DEPS | not applicable |
| 1170 | PR | MERGED | chore(deps): bump actions/checkout from 6 to 7 | DEPS | not applicable |
| 1171 | PR | CLOSED | chore(deps): bump gradle-wrapper from 8.13 to 9.6.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1172 | PR | CLOSED | chore(deps): bump com.google.crypto.tink:tink-android from 1.21.0 to 1.22.0 in /flutter\_secure\_storage/android | ESP-TINK | avoided by design |
| 1173 | PR | CLOSED | chore(deps): bump gradle-wrapper from 9.5.1 to 9.6.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1174 | PR | CLOSED | chore(deps): bump gradle-wrapper from 8.13 to 9.6.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1175 | PR | MERGED | chore(deps): bump gradle-wrapper from 9.5.1 to 9.6.1 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1176 | issue | CLOSED | macOS documentation should warn that enabling Keychain Sharing requires provisioning and free Apple Developer | DOCS | requirement |
| 1177 | PR | MERGED | fix: handle missing default keyring on linux | LINUX-KEYRING | requirement |
| 1178 | PR | MERGED | chore(deps): bump com.android.application from 9.2.1 to 9.3.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1179 | PR | MERGED | chore(deps): bump com.google.crypto.tink:tink-android from 1.21.0 to 1.23.0 in /flutter\_secure\_storage/android | ESP-TINK | avoided by design |
| 1180 | PR | MERGED | chore(deps): bump org.jetbrains.kotlin.android from 2.3.21 to 2.4.10 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 1181 | issue | CLOSED | \[linux\] xdg:schema (Libsecret schema name) attribute is being set incorrectly | LINUX-KEYRING | requirement |
| 1182 | PR | CLOSED | refactor(linux): communicate with Secret Service (org.freedesktop.secrets) directly over D-Bus | LINUX-VERIFY | requirement |
| 1183 | PR | MERGED | fix(darwin): restore access to items written before 0.3.0's SecAccess… | APP-QUERY | requirement |
| 1184 | issue | CLOSED | Can I upgrade from flutter\_secure\_storage: 9.2.4 to the latest flutter\_secure\_storage: 10.3.1? | LEGACY-VERSION | avoided by design |
| 1185 | PR | MERGED | ci: repository template files and release please | DEPS | not applicable |
| 1186 | PR | MERGED | chore(deps): bump googleapis/release-please-action from 4 to 5 | DEPS | not applicable |
| 1187 | PR | CLOSED | chore(develop): release flutter\_secure\_storage\_platform\_interface 3.0.0 | DEPS | not applicable |
| 1188 | PR | CLOSED | chore(develop): release flutter\_secure\_storage 12.0.0-beta.1 | DEPS | not applicable |
| 1189 | PR | CLOSED | chore(develop): release flutter\_secure\_storage\_web 3.0.0 | DEPS | not applicable |
| 1190 | PR | CLOSED | chore(develop): release flutter\_secure\_storage\_linux 4.0.0 | DEPS | not applicable |
| 1191 | PR | CLOSED | chore(develop): release flutter\_secure\_storage\_darwin 1.0.0 | DEPS | not applicable |
| 1192 | PR | CLOSED | chore(develop): release flutter\_secure\_storage\_windows 5.0.0 | DEPS | not applicable |
| 1193 | PR | MERGED | chore: correct release-please bootstrap tracking | DEPS | not applicable |
| 1194 | PR | CLOSED | chore(develop): release flutter\_secure\_storage\_platform\_interface 3.0.0 | DEPS | not applicable |
| 1195 | PR | MERGED | fix: publish workflow tag patterns never matched real tags | DEPS | not applicable |
| 1196 | PR | CLOSED | chore(develop): release flutter\_secure\_storage\_windows 5.0.0 | DEPS | not applicable |
| 1197 | PR | CLOSED | chore(develop): release flutter\_secure\_storage\_web 3.0.0 | DEPS | not applicable |
| 1198 | PR | MERGED | chore(develop): release flutter\_secure\_storage 11.0.0 | DEPS | not applicable |
| 1199 | PR | CLOSED | chore(develop): release flutter\_secure\_storage\_darwin 0.4.2 | DEPS | not applicable |
| 1200 | PR | CLOSED | chore(develop): release flutter\_secure\_storage\_linux 4.0.0 | DEPS | not applicable |
| 1201 | PR | CLOSED | chore(develop): release flutter\_secure\_storage\_web 3.0.0 | DEPS | not applicable |
| 1202 | PR | CLOSED | chore(develop): release flutter\_secure\_storage\_linux 4.0.0 | DEPS | not applicable |
| 1203 | issue | CLOSED | \[linux\] support XDG Desktop Secret Portal (org.freedesktop.portal.Secret) | LINUX-KEYRING | requirement |
| 1204 | PR | CLOSED | feat(linux): support for XDG Desktop Secret Portal (org.freedesktop.portal.Secret) | LINUX-KEYRING | requirement |
| 1205 | PR | MERGED | chore(develop): release flutter\_secure\_storage\_platform\_interface 2.0.3 | DEPS | not applicable |
| 1206 | PR | MERGED | chore(develop): release flutter\_secure\_storage\_linux 3.0.2 | DEPS | not applicable |
| 1207 | PR | MERGED | fix(linux): return structured libsecret error codes | LINUX-KEYRING | requirement |
| 1208 | issue | CLOSED | java.lang.NoSuchFieldError from broken OEM Keystore escapes catch (Exception) and crashes the app (Vivo / Andr | AND-LOAD | requirement |
| 1209 | PR | MERGED | fix(android): catch Throwable on worker thread so keystore Errors don't crash the app | AND-LOAD | requirement |
| 1210 | PR | MERGED | chore(deps): bump actions/stale from 10 to 11 | DEPS | not applicable |
| 1211 | PR | MERGED | chore(deps): bump com.android.application from 9.3.0 to 9.3.1 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1213 | PR | MERGED | chore(deps): bump actions/deploy-pages from 4 to 5 | DEPS | not applicable |
| 1214 | PR | MERGED | chore(deps): bump actions/upload-pages-artifact from 4 to 5 | DEPS | not applicable |
| 1215 | PR | MERGED | chore(deps): bump actions/configure-pages from 5 to 6 | DEPS | not applicable |
| 1216 | PR | MERGED | chore: promote release(s) to master | DEPS | not applicable |
| 1217 | PR | CLOSED | chore(master): release flutter\_secure\_storage\_linux 3.0.2 | DEPS | not applicable |
| 1218 | PR | CLOSED | chore(master): release flutter\_secure\_storage 11.1.0-beta.1 | DEPS | not applicable |
| 1219 | PR | MERGED | ci: gate master promotion on no other pending release PRs | DEPS | not applicable |
| 1220 | PR | MERGED | docs: add Changelog header, fix platform\_interface entry order | DEPS | not applicable |
| 1221 | PR | MERGED | docs: add Changelog header to linux and main package | DEPS | not applicable |
| 1222 | PR | MERGED | chore: promote release(s) to master | DEPS | not applicable |
| 1223 | PR | MERGED | fix(linux): skip D-Bus keyring warmup when sandboxed | LINUX-KEYRING | requirement |
| 1224 | issue | CLOSED | Version ^11.0.0 doesn't respect docs Android SDK constraint | AND-PKG | not applicable |
| 1225 | issue | CLOSED | Add migration path for encryptedSharedPreferences=true (Tink) → 11.x (currently silently wipes stored data, no | CRYPTO-LEGACY | avoided by design |
| 1226 | PR | CLOSED | chore(deps): bump gradle-wrapper from 8.13 to 9.7.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1227 | PR | CLOSED | chore(deps): bump gradle-wrapper from 9.6.1 to 9.7.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1228 | PR | MERGED | chore(deps): bump toshimaru/auto-author-assign from 3.0.3 to 3.1.0 | DEPS | not applicable |
| 1229 | issue | CLOSED | \[iOS\] Stored value disappears on some iOS devices | APP-UPSERT | requirement |
| 1230 | issue | CLOSED | Linux: the\_schema.name dangles after setLabel() — items stored with a garbage xdg:schema, orphan detection can | LINUX-KEYRING | requirement |
| 1231 | PR | MERGED | test: add regression test for storageNamespace data loss (#1126) | AND-STORE-ID | requirement |
| 1232 | PR | CLOSED | chore(deps): bump com.android.application from 9.3.1 to 9.3.2 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1233 | PR | MERGED | chore(deps): bump gradle-wrapper from 8.13 to 9.7.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1234 | PR | CLOSED | chore(deps): bump gradle-wrapper from 9.6.1 to 9.7.1 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1235 | issue | CLOSED | v11 removed APIs leave no migration path for direct upgrades from v9 or lower. App users who skip app versions | CRYPTO-LEGACY | avoided by design |
| 1236 | PR | MERGED | fix(android): use flutter.compileSdkVersion instead of pinning compileSdk | NA-BUILD | not applicable |
| 1237 | issue | CLOSED | After upgrade from v9 to v11 data is lost, but so is after downgrade from v11 to v10 | CRYPTO-LEGACY | avoided by design |
| 1238 | PR | CLOSED | chore(deps): bump com.android.application from 9.3.1 to 9.4.0 in /flutter\_secure\_storage/example/android | DEPS | not applicable |
| 1239 | PR | MERGED | chore(deps): bump actions/setup-java from 5 to 6 | DEPS | not applicable |
| 1240 | issue | CLOSED | Android Zero-Tap Sign-In Requirement | LAST-MISC | no action |
| 1241 | PR | MERGED | fix(darwin): skip synchronizable keychain queries when the data protection keychain is off | APP-SYNC | avoided by design |
| 1242 | PR | MERGED | chore: update default files to match standard, add cooldown period for dependency updates | LAST-DEPS | not applicable |
| 1243 | PR | MERGED | feat: add checkUpgradeStatus() to check for exisiting data after a missing or failed v10 migration | CRYPTO-LEGACY | avoided by design |
| 1244 | PR | MERGED | chore(deps): bump the example-android group in /flutter\_secure\_storage/example/android with 2 updates | DEPS | not applicable |
| 1245 | PR | MERGED | chore(develop): release flutter\_secure\_storage\_platform\_interface 2.1.0 | DEPS | not applicable |
| 1246 | PR | MERGED | chore(develop): release flutter\_secure\_storage 11.1.0 | DEPS | not applicable |
| 1247 | PR | MERGED | fix: require flutter\_secure\_storage\_platform\_interface ^2.1.0 for checkUpgradeStatus | CRYPTO-LEGACY | avoided by design |
| 1248 | PR | MERGED | chore: promote release(s) to master | DEPS | not applicable |
| 1249 | PR | MERGED | fix(linux)!: stop the xdg:schema attribute from going stale after relabeling | LINUX-KEYRING | requirement |
| 1250 | PR | MERGED | chore(develop): release flutter\_secure\_storage\_darwin 0.4.2 | DEPS | not applicable |
| 1251 | PR | MERGED | chore(develop): release flutter\_secure\_storage\_linux 3.0.3 | DEPS | not applicable |
| 1252 | PR | MERGED | chore: promote release(s) to master | DEPS | not applicable |
| 1253 | PR | MERGED | chore(develop): release flutter\_secure\_storage\_linux 4.0.0-beta.1 | DEPS | not applicable |
| 1254 | PR | CLOSED | chore(develop): release flutter\_secure\_storage 11.1.1 | DEPS | not applicable |
| 1255 | issue | CLOSED | isBiometricAlgorithm fails because .toString() returns object hash instead of enum name | CRYPTO-LEGACY | avoided by design |
| 1256 | PR | MERGED | fix(android): read saved key-cipher marker instead of toString() on a KeyCipher | CRYPTO-LEGACY | avoided by design |
| 1257 | PR | MERGED | fix(android): read saved key-cipher marker instead of toString() on a KeyCipher | CRYPTO-LEGACY | avoided by design |
| 1258 | PR | MERGED | chore(develop): release flutter\_secure\_storage 11.1.1 | DEPS | not applicable |
| 1259 | issue | CLOSED | Documentation Request: Direct upgrade from v9.x to v10.x using storageNamespace causes BAD\_DECRYPT | CRYPTO-LEGACY | avoided by design |
| 1260 | PR | MERGED | fix: legacy namespace key recovery biometric | CRYPTO-LEGACY | avoided by design |
| 1261 | PR | MERGED | fix: biometric multi instance race | AND-SERIAL | requirement |
| 1262 | PR | MERGED | chore(develop): release flutter\_secure\_storage 11.2.0 | DEPS | not applicable |
| 1263 | PR | MERGED | chore(develop): release flutter\_secure\_storage\_platform\_interface 2.1.1 | DEPS | not applicable |
| 1264 | PR | MERGED | feat(android): requireBiometricsPerOperation flag | AND-BIO-KEY | requirement |
| 1265 | PR | MERGED | fix(android): deleteall clearing data from other stores that share a sharedPreferencesName/storageNamespace | AND-STORE-ID | requirement |
| 1266 | PR | MERGED | fix(android): scope deleteAll to the key prefix instead of clearing the file | AND-STORE-ID | requirement |
| 1267 | PR | MERGED | fix(android): don't hang when biometric negative button is tapped | AND-BIO-PROMPT | requirement |
| 1268 | PR | MERGED | fix(android): don't hang when biometric negative button is tapped | AND-BIO-PROMPT | requirement |
| 1269 | PR | MERGED | fix(darwin): find keychain items across accessibility levels | APP-QUERY | requirement |
| 1270 | PR | MERGED | docs: warn that macOS Keychain Sharing requires provisioning | DOCS | requirement |
| 1271 | PR | MERGED | fix(android): recover biometric-protected storage on post-auth cipher failure | AND-BIO-KEY | requirement |
| 1272 | PR | MERGED | chore(develop): release flutter\_secure\_storage\_darwin 0.4.3 | DEPS | not applicable |
| 1273 | PR | MERGED | docs: add alternative Linux implementations in README | LINUX-DOC | requirement |
| 1274 | PR | MERGED | chore(deps): bump org.jetbrains.kotlin.android from 2.4.10 to 2.4.20 in /flutter\_secure\_storage/example/androi | DEPS | not applicable |
| 1275 | PR | MERGED | docs: improve Linux section in README of flutter\_secure\_storage | LINUX-DOC | requirement |
| 1276 | issue | OPEN | Log error on app open: Rejecting re-init on previously-failed class com.it\_nomads.fluttersecurestorage.Flutter | AND-LOAD | requirement |
| 1277 | issue | OPEN | \[Feature Request\] Add Swift Package Manager support for flutter\_secure\_storage | APP-PKG | not applicable |
| 1278 | issue | CLOSED | strongBiometricOnly read/write/delete hangs forever when the BiometricPrompt negative button is tapped (regres | AND-BIO-PROMPT | requirement |
| 1279 | PR | MERGED | fix(android): don't hang when strongBiometricOnly negative button is tapped | AND-BIO-PROMPT | requirement |
| 1280 | PR | MERGED | deps: bump org.robolectric:robolectric from 4.16.1 to 4.17 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1281 | PR | CLOSED | chore(deps): bump com.android.application from 9.4.0 to 9.4.1 in /flutter\_secure\_storage/example/android in th | DEPS | not applicable |
| 1282 | issue | OPEN | resetOnError recurses until the process dies when the fresh key also fails (Samsung, after biometric enrollmen | AND-RESET | requirement |
| 1283 | issue | OPEN | MissingPluginException IOS 27 | AND-LOAD | requirement |
| 1284 | PR | OPEN | fix(android): bound resetOnError re-initialization to a single attempt | AND-RESET | requirement |
| 1285 | issue | OPEN | Android: biometric key permanently invalidated after enrollment change cannot be recovered or deleted when req | AND-BIO-KEY | requirement |
| 1286 | PR | CLOSED | chore(deps): bump the example-android group across 1 directory with 2 updates | DEPS | not applicable |
| 1287 | PR | CLOSED | deps: bump gradle-wrapper from 9.7.1 to 9.8.0 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1288 | PR | MERGED | ci: stop dependabot pinning the example's path dependency | DEPS | not applicable |
| 1289 | issue | OPEN | \[Web\] Concurrent writes on a fresh storage overwrite the encryption key, leaving values undecryptable (Operati | WEB-KEY | requirement |
| 1290 | PR | OPEN | fix(linux): verify keyring stores and surface rejected libsecret calls | LINUX-VERIFY | requirement |
| 1291 | PR | CLOSED | deps: bump gradle-wrapper from 9.7.1 to 9.8.1 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1292 | PR | OPEN | chore(deps): bump futureware-tech/simulator-action from 5 to 6 in the github-actions group across 1 directory | DEPS | not applicable |
| 1293 | issue | OPEN | Android: 9.2.4 (encryptedSharedPreferences: true) → 10.3.4 with migrateWithBackup: true skips the ESP migratio | CRYPTO-LEGACY | avoided by design |
| 1294 | PR | OPEN | fix(android): recover per-operation biometric storage from an invalidated key | AND-BIO-KEY | requirement |
| 1295 | PR | OPEN | chore(deps): bump the example-android group across 1 directory with 3 updates | DEPS | not applicable |
| 1296 | PR | OPEN | feat(android): add opt-in legacy secure storage migration | ESP-TINK | avoided by design |
| 1297 | PR | OPEN | fix(web): use one encryption key when first writes run at the same time | WEB-KEY | requirement |
| 1298 | issue | OPEN | Android: transient Keystore error makes createRSAKeysIfNeeded replace the RSA pair — all data lost even with r | AND-READ | requirement |
| 1299 | PR | OPEN | fix(android): serialize initialization across engines sharing a key store | AND-SERIAL | requirement |
| 1300 | PR | CLOSED | fix(android): keep encrypting when EncryptedSharedPreferences fails to open in compat mode | ESP-TINK | avoided by design |
| 1301 | PR | OPEN | deps: bump com.google.crypto.tink:tink-android from 1.23.0 to 1.24.0 in /flutter\_secure\_storage/android | ESP-TINK | avoided by design |
| 1302 | PR | OPEN | deps: bump gradle-wrapper from 9.7.1 to 9.8.2 in /flutter\_secure\_storage/android | DEPS | not applicable |
| 1303 | issue | OPEN | requireBiometricsPerOperation: true is ignored after a cipher migration — storageCipher stays cached for the p | AND-BIO-KEY | requirement |
| 1304 | PR | OPEN | chore(develop): release flutter\_secure\_storage 11.2.1 | DEPS | not applicable |
| 1305 | PR | MERGED | chore: add analyzer excludes from Flutter 3.47 | DEPS | not applicable |
| 1306 | PR | OPEN | fix(android): preserve RSA keys on incomplete keystore reads | AND-READ | requirement |
| 1307 | issue | OPEN | Swift Package Manager Support: Add FlutterFramework Dependency | APP-PKG | not applicable |

