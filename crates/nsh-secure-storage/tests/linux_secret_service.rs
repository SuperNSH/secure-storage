#![cfg(target_os = "linux")]

//! The Linux backend on a real Secret Service. Runs only in the Docker image (`tools/linux-test`),
//! where a throwaway keyring is unlocked for the run. Set `NSH_SECURE_STORAGE_LINUX_TESTS=1` to run.

use std::{future::Future, sync::Arc};

use nsh_secure_storage::{
    LinuxBackend, PlatformOptions, ReadOptions, SecureStorage, SecureStorageError,
    SecureStorageOptions, WriteOptions,
};

fn enabled() -> bool {
    if std::env::var_os("NSH_SECURE_STORAGE_LINUX_TESTS").as_deref() != Some("1".as_ref()) {
        eprintln!("skipped: set NSH_SECURE_STORAGE_LINUX_TESTS=1 inside the Docker image");
        return false;
    }
    true
}

fn block_on<T>(future: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime for the blocking pool")
        .block_on(future)
}

/// A store on the real Secret Service. Each test uses its own name, and removes it at the end.
fn storage(account: &str) -> SecureStorage {
    let mut platform = PlatformOptions::default();
    platform.linux.account_name = account.to_owned();
    let mut options = SecureStorageOptions::default();
    options.platform = platform;
    SecureStorage::with_backend(Arc::new(LinuxBackend::system()), options)
}

fn read(storage: SecureStorage, key: &str) -> Result<Option<String>, SecureStorageError> {
    block_on(storage.read(key, &ReadOptions::default()))
        .map(|value| value.map(|value| value.to_string()))
}

fn write(storage: SecureStorage, key: &str, value: &str) -> Result<(), SecureStorageError> {
    block_on(storage.write(key, Some(value), &WriteOptions::default()))
}

/// Removes every value of `storage` when the test ends, even when it fails.
struct Cleanup(SecureStorage);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = block_on(self.0.delete_all(&Default::default()));
    }
}

fn unique(name: &str) -> String {
    format!("nsh-test-{name}-{}", std::process::id())
}

#[test]
fn linux_secret_service_round_trip() {
    if !enabled() {
        return;
    }
    let store = storage(&unique("round-trip"));
    let _clean = Cleanup(store);
    assert_eq!(read(store, "token").unwrap(), None);
    write(store, "token", "abc").unwrap();
    assert_eq!(read(store, "token").unwrap().as_deref(), Some("abc"));
    write(store, "token", "def").unwrap();
    assert_eq!(read(store, "token").unwrap().as_deref(), Some("def"));
    write(store, "empty", "").unwrap();
    assert_eq!(read(store, "empty").unwrap().as_deref(), Some(""));
    assert!(block_on(store.contains_key("token", &Default::default())).unwrap());
    block_on(store.delete("token", &Default::default())).unwrap();
    assert_eq!(read(store, "token").unwrap(), None);
    block_on(store.delete("token", &Default::default())).expect("deleting an absent key succeeds");
}

#[test]
fn linux_secret_service_read_all_and_delete_all_are_scoped() {
    if !enabled() {
        return;
    }
    let name = unique("scoped");
    let mine = storage(&name);
    let other = storage(&format!("{name}-other"));
    let _clean_mine = Cleanup(mine);
    let _clean_other = Cleanup(other);
    write(mine, "b", "2").unwrap();
    write(mine, "a", "1").unwrap();
    write(other, "a", "other").unwrap();

    let all: Vec<(String, String)> = block_on(mine.read_all(&Default::default()))
        .unwrap()
        .into_iter()
        .map(|(key, value)| (key, value.to_string()))
        .collect();
    assert_eq!(
        all,
        vec![
            ("a".to_owned(), "1".to_owned()),
            ("b".to_owned(), "2".to_owned()),
        ],
        "read_all lists this store only, in key order"
    );

    block_on(mine.delete_all(&Default::default())).unwrap();
    assert!(
        block_on(mine.read_all(&Default::default()))
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        read(other, "a").unwrap().as_deref(),
        Some("other"),
        "delete_all leaves a store whose name is different"
    );
}

#[test]
fn linux_secret_service_special_characters_round_trip() {
    if !enabled() {
        return;
    }
    let store = storage(&unique("special"));
    let _clean = Cleanup(store);
    let keys = [
        "colon:name",
        "slash/name",
        "back\\slash",
        "..",
        "quote\"and'apostrophe",
        "ünïcode",
        "emoji 🔑",
    ];
    for key in keys {
        let value = format!("value of {key} with \n newline and \t tab 🔐");
        write(store, key, &value).unwrap();
        assert_eq!(read(store, key).unwrap().as_deref(), Some(value.as_str()));
    }
}

#[test]
fn linux_secret_service_one_mib_round_trips() {
    if !enabled() {
        return;
    }
    let store = storage(&unique("big"));
    let _clean = Cleanup(store);
    let value: String = (0..(1 << 20))
        .map(|i| char::from(b'a' + (i % 26) as u8))
        .collect();
    write(store, "big", &value).unwrap();
    assert_eq!(read(store, "big").unwrap().as_deref(), Some(value.as_str()));
}
