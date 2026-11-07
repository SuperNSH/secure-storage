//! The iOS Keychain backend, on a simulator's real Keychain. `tools/ios-test/run.sh` runs this,
//! because the binary must carry simulated entitlements.

#![cfg(target_os = "ios")]

use std::future::Future;

use nsh_secure_storage::{
    DeleteAllOptions, DeleteOptions, KeychainAccessibility, PlatformOptions, ReadAllOptions,
    ReadOptions, SecureStorage, SecureStorageOptions, WriteOptions,
};

fn run<T>(future: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime for the blocking pool")
        .block_on(future)
}

fn storage_at(account: &str, accessibility: KeychainAccessibility) -> SecureStorage {
    let mut platform = PlatformOptions::default();
    platform.ios.apple.account_name = account.to_owned();
    platform.ios.apple.accessibility = accessibility;
    let mut options = SecureStorageOptions::default();
    options.platform = platform;
    SecureStorage::new(options)
}

fn storage(account: &str) -> SecureStorage {
    storage_at(account, KeychainAccessibility::UnlockedThisDevice)
}

fn clean(storage: SecureStorage) {
    run(storage.delete_all(&DeleteAllOptions::default())).expect("delete_all");
}

#[test]
fn ios_write_read_update_delete() {
    let store = storage("nsh-ios-round-trip");
    clean(store);
    let write = WriteOptions::default();
    let read = ReadOptions::default();
    run(store.write("k", Some("first"), &write)).expect("write");
    assert_eq!(
        run(store.read("k", &read))
            .expect("read")
            .as_deref()
            .map(String::as_str),
        Some("first")
    );
    run(store.write("k", Some("second"), &write)).expect("update");
    assert_eq!(
        run(store.read("k", &read))
            .expect("read")
            .as_deref()
            .map(String::as_str),
        Some("second")
    );
    assert!(run(store.contains_key("k", &Default::default())).expect("contains"));
    run(store.delete("k", &DeleteOptions::default())).expect("delete");
    assert!(run(store.read("k", &read)).expect("read").is_none());
    assert!(!run(store.contains_key("k", &Default::default())).expect("contains"));
}

#[test]
fn ios_read_all_and_delete_all_stay_in_their_account() {
    let (mine, other) = (storage("nsh-ios-mine"), storage("nsh-ios-other"));
    clean(mine);
    clean(other);
    let write = WriteOptions::default();
    run(mine.write("a", Some("1"), &write)).expect("write");
    run(mine.write("b/ü é", Some(""), &write)).expect("write");
    run(other.write("c", Some("3"), &write)).expect("write");
    let read = ReadAllOptions::default();
    let all = run(mine.read_all(&read)).expect("read_all");
    assert_eq!(all.len(), 2);
    assert_eq!(
        all.get("b/ü é").map(|v| v.as_str()),
        Some(""),
        "an empty string is a value"
    );
    run(mine.delete_all(&DeleteAllOptions::default())).expect("delete_all");
    assert!(run(mine.read_all(&read)).expect("read_all").is_empty());
    assert_eq!(run(other.read_all(&read)).expect("read_all").len(), 1);
    clean(other);
}

#[test]
fn ios_finds_an_item_across_accessibility_levels() {
    let first = storage_at("nsh-ios-levels", KeychainAccessibility::Unlocked);
    let other = storage_at(
        "nsh-ios-levels",
        KeychainAccessibility::FirstUnlockThisDevice,
    );
    clean(first);
    run(first.write("k", Some("v"), &WriteOptions::default())).expect("write");
    let found = run(other.read("k", &ReadOptions::default())).expect("read");
    assert_eq!(found.as_deref().map(String::as_str), Some("v"));
    clean(first);
}

#[test]
fn ios_long_value_round_trips() {
    let store = storage("nsh-ios-long");
    clean(store);
    let value = "x".repeat(1024 * 1024);
    run(store.write("k", Some(&value), &WriteOptions::default())).expect("write");
    let got = run(store.read("k", &ReadOptions::default()))
        .expect("read")
        .expect("a value");
    assert_eq!(got.len(), value.len());
    clean(store);
}
