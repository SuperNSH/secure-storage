//! The macOS Keychain backend, against a throwaway keychain file in a temp folder. The login
//! keychain is never opened: every test names its own file and deletes it afterwards.

#![cfg(target_os = "macos")]

use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::Once,
};

use core_foundation::string::CFStringRef;
use core_foundation::{
    base::{CFType, CFTypeRef, TCFType},
    date::CFDate,
};
use nsh_secure_storage::{
    AccessControlFlag, AuthenticationUi, ErrorKind, KeychainAccessibility, MacOsOptions,
    PlatformOptions, ReadOptions, SecureStorage, SecureStorageOptions, WriteOptions,
};
use security_framework::{
    item::{ItemClass, ItemSearchOptions, Limit, SearchResult},
    os::macos::keychain::{CreateOptions, SecKeychain},
};

// `security-framework-sys` does not export this attribute name, so the test declares it.
#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    static kSecAttrCreationDate: CFStringRef;
}

/// Turns keychain interaction off for this whole test process, once, before any keychain call.
/// Then no test can open a system dialog: a call that would ask fails with `errSecInteractionNotAllowed`.
fn no_dialogs() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let guard = SecKeychain::disable_user_interaction().expect("interaction can be turned off");
        // Kept off for the rest of the process: dropping the guard would turn interaction back on.
        std::mem::forget(guard);
        assert!(!SecKeychain::user_interaction_allowed().expect("interaction state is readable"));
    });
}

/// A keychain file that exists for one test, and is removed when the test ends.
struct ScratchKeychain {
    path: PathBuf,
}

impl ScratchKeychain {
    fn create(name: &str) -> Self {
        no_dialogs();
        // A run killed mid-test skips Drop, so earlier leftovers are removed here (files only).
        if let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) {
            for entry in entries.flatten() {
                let file = entry.file_name();
                let file = file.to_string_lossy();
                // Tests run in parallel, so leave this process's own keychains alone.
                let own = format!("-{}.keychain-db", std::process::id());
                if file.starts_with("nsh-secure-storage-")
                    && file.ends_with(".keychain-db")
                    && !file.ends_with(&own)
                {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
        let path = std::env::temp_dir().join(format!(
            "nsh-secure-storage-{name}-{}.keychain-db",
            std::process::id()
        ));
        CreateOptions::new()
            .password("throwaway-test-password")
            .create(&path)
            .expect("the throwaway keychain is created");
        Self { path }
    }
}

impl Drop for ScratchKeychain {
    fn drop(&mut self) {
        let _ = Command::new("security")
            .arg("delete-keychain")
            .arg(&self.path)
            .output();
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Options that target `keychain` and `service`, using the file keychain.
fn options(keychain: &Path, service: &str) -> PlatformOptions {
    let mut platform = PlatformOptions::default();
    platform.macos = MacOsOptions::default();
    platform.macos.uses_data_protection_keychain = false;
    platform.macos.keychain_path = Some(keychain.to_path_buf());
    platform.macos.apple.account_name = service.to_owned();
    platform
}

fn store(keychain: &Path, service: &str) -> SecureStorage {
    SecureStorage::new(with_platform(options(keychain, service)))
}

/// The store options for `platform`. The options struct is `#[non_exhaustive]`, so it is set field by field.
fn with_platform(platform: PlatformOptions) -> SecureStorageOptions {
    let mut options = SecureStorageOptions::default();
    options.platform = platform;
    options
}

/// Runs one Keychain future on a runtime, so `nsh_core::blocking` has its pool.
fn run<T>(future: impl std::future::Future<Output = T>) -> T {
    no_dialogs();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime for the blocking pool")
        .block_on(future)
}

fn read(storage: SecureStorage, key: &str) -> Option<String> {
    run(storage.read(key, &ReadOptions::default()))
        .unwrap()
        .map(|value| value.to_string())
}

fn write(storage: SecureStorage, key: &str, value: &str) {
    run(storage.write(key, Some(value), &WriteOptions::default())).unwrap();
}

#[test]
fn macos_keychain_round_trip() {
    let scratch = ScratchKeychain::create("round-trip");
    let storage = store(&scratch.path, "nss-round-trip");
    assert_eq!(read(storage, "token"), None);
    write(storage, "token", "first");
    assert_eq!(read(storage, "token").as_deref(), Some("first"));
    assert!(run(storage.contains_key("token", &Default::default())).unwrap());
    run(storage.delete("token", &Default::default())).unwrap();
    assert_eq!(read(storage, "token"), None);
    assert!(!run(storage.contains_key("token", &Default::default())).unwrap());
}

/// The creation date of the item of `service` and `account` in `keychain`, as the keychain sets it.
fn creation_date(keychain: &Path, service: &str, account: &str) -> f64 {
    let keychain = SecKeychain::open(keychain).expect("the throwaway keychain opens");
    let found = ItemSearchOptions::new()
        .class(ItemClass::generic_password())
        .keychains(&[keychain])
        .service(service)
        .account(account)
        .load_attributes(true)
        .limit(Limit::All)
        .search()
        .expect("the item is searched for");
    let [SearchResult::Dict(attributes)] = found.as_slice() else {
        panic!("exactly one item is expected");
    };
    let date = attributes
        // SAFETY: Security exports this constant once the framework is loaded.
        .find(unsafe { kSecAttrCreationDate } as *const std::ffi::c_void)
        .expect("the item has a creation date");
    // SAFETY: the keychain returns a CFDate for this attribute, and the dictionary keeps it alive.
    let date = unsafe { CFType::wrap_under_get_rule(*date as CFTypeRef) };
    date.downcast::<CFDate>()
        .expect("the creation date is a CFDate")
        .abs_time()
}

#[test]
fn apple_write_overwrites_without_delete() {
    // A delete then an add would give the item a new creation date; an update keeps the date.
    let scratch = ScratchKeychain::create("update");
    let storage = store(&scratch.path, "nss-update");
    write(storage, "k", "one");
    let created = creation_date(&scratch.path, "nss-update", "k");
    std::thread::sleep(std::time::Duration::from_millis(1100));
    write(storage, "k", "two");
    assert_eq!(read(storage, "k").as_deref(), Some("two"));
    assert_eq!(
        creation_date(&scratch.path, "nss-update", "k"),
        created,
        "an update must keep the item: a delete then add gives it a new creation date"
    );
    let all = run(storage.read_all(&Default::default())).unwrap();
    assert_eq!(all.len(), 1, "an update must not add a second item");
}

#[test]
fn apple_transient_failure_keeps_item() {
    // A locked keychain is a transient failure: a read, a write and a delete fail, and the item stays.
    let scratch = ScratchKeychain::create("transient");
    write(store(&scratch.path, "nss-transient"), "k", "keep-me");
    let mut platform = options(&scratch.path, "nss-transient");
    platform.macos.apple.authentication_ui = Some(AuthenticationUi::Fail);
    let guarded = SecureStorage::new(with_platform(platform));

    lock_keychain(&scratch.path);
    let read_error = run(guarded.read("k", &ReadOptions::default())).unwrap_err();
    assert_eq!(read_error.kind(), ErrorKind::Locked);
    assert!(run(guarded.write("k", Some("replaced"), &WriteOptions::default())).is_err());
    assert!(run(guarded.delete("k", &Default::default())).is_err());
    unlock_keychain(&scratch.path);

    assert_eq!(
        read(store(&scratch.path, "nss-transient"), "k").as_deref(),
        Some("keep-me"),
        "the failed write and delete left the item alone"
    );
}

fn lock_keychain(path: &Path) {
    let status = Command::new("security")
        .arg("lock-keychain")
        .arg(path)
        .status()
        .expect("the security tool runs");
    assert!(status.success(), "the throwaway keychain locks");
}

fn unlock_keychain(path: &Path) {
    let status = Command::new("security")
        .args(["unlock-keychain", "-p", "throwaway-test-password"])
        .arg(path)
        .status()
        .expect("the security tool runs");
    assert!(status.success(), "the throwaway keychain unlocks");
}

#[test]
fn macos_read_all_is_scoped_to_the_account_name() {
    let scratch = ScratchKeychain::create("scope");
    let mine = store(&scratch.path, "nss-scope-mine");
    let other = store(&scratch.path, "nss-scope-other");
    write(mine, "a", "1");
    write(mine, "b", "2");
    write(other, "x", "9");

    let listed = run(mine.read_all(&Default::default())).unwrap();
    let keys: Vec<&str> = listed.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        ["a", "b"],
        "read_all must list only this account name"
    );

    run(mine.delete_all(&Default::default())).unwrap();
    assert!(run(mine.read_all(&Default::default())).unwrap().is_empty());
    assert_eq!(
        read(other, "x").as_deref(),
        Some("9"),
        "delete_all must not touch another service"
    );
}

#[test]
fn apple_read_finds_item_written_with_other_accessibility() {
    // Upstream #1269: an item written at one level must be found by a read that names another.
    let scratch = ScratchKeychain::create("accessibility");
    let mut written = options(&scratch.path, "nss-accessibility");
    written.macos.apple.accessibility = KeychainAccessibility::Unlocked;
    run(SecureStorage::new(with_platform(written)).write(
        "k",
        Some("found"),
        &WriteOptions::default(),
    ))
    .unwrap();

    let mut reading = options(&scratch.path, "nss-accessibility");
    reading.macos.apple.accessibility = KeychainAccessibility::FirstUnlockThisDevice;
    let found =
        run(SecureStorage::new(with_platform(reading)).read("k", &ReadOptions::default())).unwrap();
    assert_eq!(found.as_deref().map(String::as_str), Some("found"));
}

#[test]
fn large_value_round_trips_on_every_platform() {
    let scratch = ScratchKeychain::create("values");
    let storage = store(&scratch.path, "nss-values");
    let keys = [
        "with space",
        "slash/and\\back",
        "emoji-🔐",
        "quote\"and'apostrophe",
        "..",
        "colon:and;semi",
    ];
    for key in keys {
        write(storage, key, key);
    }
    for key in keys {
        assert_eq!(read(storage, key).as_deref(), Some(key));
    }
    let long_key = "k".repeat(300);
    write(storage, &long_key, "long-key");
    assert_eq!(read(storage, &long_key).as_deref(), Some("long-key"));
    write(storage, "empty", "");
    assert_eq!(read(storage, "empty").as_deref(), Some(""));
    let long = "y".repeat(1 << 20);
    write(storage, "long", &long);
    assert_eq!(read(storage, "long").as_deref(), Some(long.as_str()));
}

#[test]
fn apple_locked_device_returns_coded_error() {
    let scratch = ScratchKeychain::create("locked");
    let mut platform = options(&scratch.path, "nss-locked");
    platform.macos.apple.authentication_ui = Some(AuthenticationUi::Fail);
    let storage = SecureStorage::new(with_platform(platform));
    write(store(&scratch.path, "nss-locked"), "k", "secret-value");

    let status = Command::new("security")
        .arg("lock-keychain")
        .arg(&scratch.path)
        .status()
        .expect("the security tool runs");
    assert!(status.success(), "the throwaway keychain locks");

    let error = run(storage.read("k", &ReadOptions::default())).unwrap_err();
    assert_eq!(
        error.kind(),
        ErrorKind::Locked,
        "a locked keychain with no prompt allowed"
    );
    assert_eq!(error.code(), "ERR_SECURE_STORAGE_LOCKED");
    assert!(!format!("{error:?}").contains("secret-value"));
}

#[test]
fn macos_data_protection_without_entitlement_names_the_fix() {
    // An unsigned test binary has no keychain access group, so the data protection keychain refuses it.
    let mut platform = PlatformOptions::default();
    platform.macos.apple.account_name = "nss-entitlement".to_owned();
    let storage = SecureStorage::new(with_platform(platform));
    let error = run(storage.write("k", Some("v"), &WriteOptions::default())).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::MissingEntitlement);
    assert_eq!(error.status(), Some(-34018));
    assert!(error.message().contains("keychain-access-groups"));
}

#[test]
fn macos_access_control_is_refused_on_the_file_keychain() {
    // The file keychain refuses access control (`errSecParam`), so the refusal is named up front.
    let scratch = ScratchKeychain::create("access-control");
    let mut platform = options(&scratch.path, "nss-access-control");
    platform.macos.apple.access_control_flags = vec![AccessControlFlag::UserPresence];
    let storage = SecureStorage::new(with_platform(platform));
    let error = run(storage.write("guarded", Some("v"), &WriteOptions::default())).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NotAvailable);
}

#[test]
fn store_names_isolate_data() {
    let scratch = ScratchKeychain::create("names");
    let first = store(&scratch.path, "nss-names-first");
    let second = store(&scratch.path, "nss-names-second");
    write(first, "same", "from-first");
    write(second, "same", "from-second");
    assert_eq!(read(first, "same").as_deref(), Some("from-first"));
    assert_eq!(read(second, "same").as_deref(), Some("from-second"));
}

#[test]
fn apple_default_accessibility_is_this_device_only() {
    assert_eq!(
        nsh_secure_storage::AppleOptions::default().accessibility,
        KeychainAccessibility::UnlockedThisDevice
    );
    assert_eq!(
        nsh_secure_storage::IosOptions::default()
            .apple
            .accessibility,
        KeychainAccessibility::UnlockedThisDevice
    );
}
