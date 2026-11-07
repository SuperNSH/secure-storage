//! The Linux backend against keyring-core's mock store, which runs on every host. The Secret
//! Service itself is exercised in `linux_secret_service.rs`, inside the Docker image.

use std::{
    any::Any,
    collections::HashMap,
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use keyring_core::{
    Credential, CredentialStore, Entry, Error as KeyringError,
    api::{CredentialApi, CredentialStoreApi},
    mock,
};
use nsh_secure_storage::{
    LinuxBackend, PlatformOptions, ReadOptions, SecureStorage, SecureStorageError,
    SecureStorageOptions, WriteOptions,
};

const ACCOUNT: &str = "com.example.app";

fn block_on<T>(future: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime for the blocking pool")
        .block_on(future)
}

fn storage(keys: &Arc<CredentialStore>, account: &str) -> SecureStorage {
    let mut platform = PlatformOptions::default();
    platform.linux.account_name = account.to_owned();
    let mut options = SecureStorageOptions::default();
    options.platform = platform;
    SecureStorage::with_backend(
        Arc::new(LinuxBackend::with_store(Arc::clone(keys))),
        options,
    )
}

fn read(storage: SecureStorage, key: &str) -> Result<Option<String>, SecureStorageError> {
    block_on(storage.read(key, &ReadOptions::default()))
        .map(|value| value.map(|value| value.to_string()))
}

fn write(storage: SecureStorage, key: &str, value: &str) -> Result<(), SecureStorageError> {
    block_on(storage.write(key, Some(value), &WriteOptions::default()))
}

/// Makes the next call on the item of `key` fail with `error`.
fn fail_next_call(keys: &Arc<CredentialStore>, account: &str, key: &str, error: KeyringError) {
    let entry = keys
        .build(account, key, None)
        .expect("the mock builds an entry");
    let cred: &mock::Cred = entry.as_any().downcast_ref().expect("a mock credential");
    cred.set_error(error);
}

/// A store whose writes can be made to vanish. A keyring that accepts a write and keeps the old
/// bytes looks like this.
struct DroppingStore {
    inner: Arc<mock::Store>,
    dropping: Arc<AtomicBool>,
}

impl DroppingStore {
    fn wrap(&self, entry: Entry) -> Entry {
        Entry::new_with_credential(Arc::new(DroppingCred {
            inner: entry,
            dropping: Arc::clone(&self.dropping),
        }))
    }
}

impl CredentialStoreApi for DroppingStore {
    fn vendor(&self) -> String {
        "dropping test store".to_owned()
    }

    fn id(&self) -> String {
        "dropping test store".to_owned()
    }

    fn build(
        &self,
        service: &str,
        user: &str,
        modifiers: Option<&HashMap<&str, &str>>,
    ) -> keyring_core::Result<Entry> {
        Ok(self.wrap(self.inner.build(service, user, modifiers)?))
    }

    fn search(&self, spec: &HashMap<&str, &str>) -> keyring_core::Result<Vec<Entry>> {
        Ok(self
            .inner
            .search(spec)?
            .into_iter()
            .map(|entry| self.wrap(entry))
            .collect())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct DroppingCred {
    inner: Entry,
    dropping: Arc<AtomicBool>,
}

impl CredentialApi for DroppingCred {
    fn set_secret(&self, secret: &[u8]) -> keyring_core::Result<()> {
        if self.dropping.load(Ordering::SeqCst) {
            return Ok(());
        }
        self.inner.set_secret(secret)
    }

    fn get_secret(&self) -> keyring_core::Result<Vec<u8>> {
        self.inner.get_secret()
    }

    fn delete_credential(&self) -> keyring_core::Result<()> {
        self.inner.delete_credential()
    }

    fn get_credential(&self) -> keyring_core::Result<Option<Arc<Credential>>> {
        Ok(None)
    }

    fn get_specifiers(&self) -> Option<(String, String)> {
        self.inner.get_specifiers()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[test]
fn linux_write_verifies_readback_and_reports_rejection() {
    let dropping = Arc::new(AtomicBool::new(false));
    let keys: Arc<CredentialStore> = Arc::new(DroppingStore {
        inner: mock::Store::new().unwrap(),
        dropping: Arc::clone(&dropping),
    });
    let store = storage(&keys, ACCOUNT);
    write(store, "a", "1").unwrap();

    dropping.store(true, Ordering::SeqCst);
    let error = write(store, "b", "2").expect_err("a write that does not read back is an error");
    assert_eq!(error.code(), "ERR_SECURE_STORAGE_BACKEND");
    assert_eq!(error.key(), Some("b"), "the error names the key");
    dropping.store(false, Ordering::SeqCst);
    assert_eq!(
        read(store, "b").unwrap(),
        None,
        "nothing was stored under b"
    );
}

#[test]
fn linux_write_keeps_other_keys_when_replace_is_dropped() {
    let dropping = Arc::new(AtomicBool::new(false));
    let keys: Arc<CredentialStore> = Arc::new(DroppingStore {
        inner: mock::Store::new().unwrap(),
        dropping: Arc::clone(&dropping),
    });
    let store = storage(&keys, ACCOUNT);
    write(store, "a", "1").unwrap();
    write(store, "b", "2").unwrap();

    dropping.store(true, Ordering::SeqCst);
    assert!(
        write(store, "a", "3").is_err(),
        "the dropped replace is reported"
    );
    dropping.store(false, Ordering::SeqCst);
    assert_eq!(
        read(store, "a").unwrap().as_deref(),
        Some("1"),
        "the old value stays"
    );
    assert_eq!(
        read(store, "b").unwrap().as_deref(),
        Some("2"),
        "other keys are kept"
    );
}

#[test]
fn linux_locked_keyring_is_coded_error() {
    let keys = mock::Store::new().unwrap();
    let keys: Arc<CredentialStore> = keys;
    let store = storage(&keys, ACCOUNT);
    write(store, "k", "v").unwrap();

    fail_next_call(
        &keys,
        ACCOUNT,
        "k",
        KeyringError::NoStorageAccess(Box::new(std::io::Error::other("locked"))),
    );
    let error = read(store, "k").expect_err("a locked keyring is an error, not a missing key");
    assert_eq!(error.code(), "ERR_SECURE_STORAGE_LOCKED");
    assert_eq!(error.key(), Some("k"));
}

/// Each keyring error, and the code it must report. Break the mapping and this fails.
#[test]
fn linux_error_mapping_is_coded() {
    type MakeError = fn() -> KeyringError;
    let table: Vec<(MakeError, &str)> = vec![
        (
            || KeyringError::PlatformFailure(Box::new(std::io::Error::other("x"))),
            "ERR_SECURE_STORAGE_BACKEND",
        ),
        (
            || KeyringError::NoStorageAccess(Box::new(std::io::Error::other("x"))),
            "ERR_SECURE_STORAGE_LOCKED",
        ),
        (
            || KeyringError::TooLong("username".to_owned(), 10),
            "ERR_SECURE_STORAGE_NOT_AVAILABLE",
        ),
        (
            || KeyringError::NoDefaultStore,
            "ERR_SECURE_STORAGE_NOT_AVAILABLE",
        ),
        (
            || KeyringError::BadEncoding(vec![0xff]),
            "ERR_SECURE_STORAGE_CORRUPT",
        ),
    ];
    for (make, code) in table {
        let keys: Arc<CredentialStore> = mock::Store::new().unwrap();
        let store = storage(&keys, ACCOUNT);
        write(store, "k", "v").unwrap();
        fail_next_call(&keys, ACCOUNT, "k", make());
        let error = write(store, "k", "w").expect_err("the failure is reported");
        assert_eq!(error.code(), code, "for {:?}", make());
    }
}

#[test]
fn store_names_isolate_data() {
    let keys: Arc<CredentialStore> = mock::Store::new().unwrap();
    let one = storage(&keys, "nsh");
    // A name that contains the other as a substring: the mock's search matches substrings.
    let two = storage(&keys, "nsh_secure_storage");
    write(one, "token", "one").unwrap();
    assert_eq!(read(two, "token").unwrap(), None);
    write(two, "token", "two").unwrap();
    assert_eq!(read(one, "token").unwrap().as_deref(), Some("one"));
}

#[test]
fn delete_all_touches_only_own_prefix() {
    let keys: Arc<CredentialStore> = mock::Store::new().unwrap();
    let one = storage(&keys, "nsh");
    let two = storage(&keys, "nsh_secure_storage");
    write(one, "a", "1").unwrap();
    write(one, "b", "2").unwrap();
    write(two, "a", "3").unwrap();

    block_on(one.delete_all(&Default::default())).unwrap();
    assert!(
        block_on(one.read_all(&Default::default()))
            .unwrap()
            .is_empty(),
        "the store's own keys are gone"
    );
    assert_eq!(
        read(two, "a").unwrap().as_deref(),
        Some("3"),
        "a store whose name contains this one keeps its keys"
    );
}

#[test]
fn read_all_returns_its_own_keys_in_order() {
    let keys: Arc<CredentialStore> = mock::Store::new().unwrap();
    let one = storage(&keys, "nsh");
    let two = storage(&keys, "nsh_secure_storage");
    write(one, "b", "2").unwrap();
    write(one, "a", "1").unwrap();
    write(two, "c", "3").unwrap();
    let all: Vec<(String, String)> = block_on(one.read_all(&Default::default()))
        .unwrap()
        .into_iter()
        .map(|(key, value)| (key, value.to_string()))
        .collect();
    assert_eq!(
        all,
        vec![
            ("a".to_owned(), "1".to_owned()),
            ("b".to_owned(), "2".to_owned()),
        ]
    );
}

#[test]
fn reserved_key_characters_round_trip() {
    let keys: Arc<CredentialStore> = mock::Store::new().unwrap();
    let store = storage(&keys, ACCOUNT);
    let long = "k".repeat(300);
    let names = [
        "colon:name",
        "slash/name",
        "back\\slash",
        "..",
        "ünïcode",
        "emoji 🔑",
        long.as_str(),
    ];
    for name in names {
        write(store, name, name).unwrap();
    }
    for name in names {
        assert_eq!(read(store, name).unwrap().as_deref(), Some(name));
    }
}

#[test]
fn long_values_round_trip() {
    let keys: Arc<CredentialStore> = mock::Store::new().unwrap();
    let store = storage(&keys, ACCOUNT);
    let value: String = (0..(1 << 20))
        .map(|i| char::from(b'a' + (i % 26) as u8))
        .collect();
    write(store, "big", &value).unwrap();
    assert_eq!(read(store, "big").unwrap().as_deref(), Some(value.as_str()));
}

#[test]
fn absent_key_is_none_and_contains_key_agrees() {
    let keys: Arc<CredentialStore> = mock::Store::new().unwrap();
    let store = storage(&keys, ACCOUNT);
    assert_eq!(read(store, "k").unwrap(), None);
    assert!(!block_on(store.contains_key("k", &Default::default())).unwrap());
    write(store, "k", "").unwrap();
    assert_eq!(read(store, "k").unwrap().as_deref(), Some(""));
    assert!(block_on(store.contains_key("k", &Default::default())).unwrap());
    block_on(store.delete("k", &Default::default())).unwrap();
    block_on(store.delete("k", &Default::default())).expect("deleting an absent key succeeds");
    assert!(!block_on(store.contains_key("k", &Default::default())).unwrap());
}
