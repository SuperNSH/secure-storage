//! The Windows backend against keyring-core's mock store and a folder in the temp directory. It
//! runs on every host: Credential Manager and the login keychain are never opened.

use std::{
    fs,
    future::Future,
    path::{Path, PathBuf},
    sync::Arc,
};

use std::collections::HashMap;

use base64::{
    Engine,
    prelude::{BASE64_STANDARD, BASE64_URL_SAFE_NO_PAD},
};
use keyring_core::{CredentialStore, Entry, Error as KeyringError, mock};
use nsh_secure_storage::{
    PlatformOptions, ReadOptions, SecureStorage, SecureStorageError, SecureStorageOptions,
    WindowsBackend, WriteOptions,
};

/// A folder for one test, removed when the test ends.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "nsh-secure-storage-win-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("the scratch folder is created");
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn block_on<T>(future: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime for the blocking pool")
        .block_on(future)
}

/// A store for `app`, on `keys` (the mock Credential Manager) and `root` (the values folder).
fn storage(keys: &Arc<CredentialStore>, root: &Path, app: &str, account: &str) -> SecureStorage {
    let mut platform = PlatformOptions::default();
    platform.windows.app_id = Some(app.to_owned());
    platform.windows.account_name = account.to_owned();
    let mut options = SecureStorageOptions::default();
    options.platform = platform;
    let backend = WindowsBackend::new(Arc::clone(keys), root.to_path_buf());
    SecureStorage::with_backend(Arc::new(backend), options)
}

fn new_keys() -> Arc<CredentialStore> {
    mock::Store::new().expect("the mock store")
}

fn read(storage: SecureStorage, key: &str) -> Result<Option<String>, SecureStorageError> {
    block_on(storage.read(key, &ReadOptions::default()))
        .map(|value| value.map(|value| value.to_string()))
}

fn write(storage: SecureStorage, key: &str, value: &str) -> Result<(), SecureStorageError> {
    block_on(storage.write(key, Some(value), &WriteOptions::default()))
}

/// The mock credential that holds the key of one store. `service` and `user` are the encoded
/// app and store names the backend uses.
fn key_entry(keys: &Arc<CredentialStore>, app: &str, account: &str) -> Entry {
    keys.build(
        &BASE64_URL_SAFE_NO_PAD.encode(app),
        &BASE64_URL_SAFE_NO_PAD.encode(account),
        None,
    )
    .expect("the mock builds an entry")
}

/// Every `.values` file under `root`.
fn values_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut folders = vec![root.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(&folder)
            .expect("the folder is readable")
            .flatten()
        {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
            } else if path.extension().is_some_and(|ext| ext == "values") {
                found.push(path);
            }
        }
    }
    found
}

#[test]
fn windows_values_file_round_trips() {
    let scratch = Scratch::new("round-trip");
    let keys = new_keys();
    let store = storage(&keys, &scratch.0, "com.example.app", "nsh_secure_storage");
    assert_eq!(read(store, "token").unwrap(), None, "an absent key is None");
    write(store, "token", "abc").unwrap();
    write(store, "empty", "").unwrap();
    assert_eq!(read(store, "token").unwrap().as_deref(), Some("abc"));
    assert_eq!(
        read(store, "empty").unwrap().as_deref(),
        Some(""),
        "an empty value is a value, not absent"
    );

    // A new handle on the same store, as after a restart, reads what was written.
    let again = storage(&keys, &scratch.0, "com.example.app", "nsh_secure_storage");
    assert_eq!(read(again, "token").unwrap().as_deref(), Some("abc"));
    let all = block_on(again.read_all(&Default::default())).unwrap();
    let all: Vec<(String, String)> = all
        .into_iter()
        .map(|(key, value)| (key, value.to_string()))
        .collect();
    assert_eq!(
        all,
        vec![
            ("empty".to_owned(), String::new()),
            ("token".to_owned(), "abc".to_owned()),
        ]
    );

    block_on(again.delete("token", &Default::default())).unwrap();
    assert_eq!(read(again, "token").unwrap(), None);
    block_on(again.delete("token", &Default::default())).expect("deleting an absent key succeeds");
    block_on(again.delete_all(&Default::default())).unwrap();
    assert!(
        block_on(again.read_all(&Default::default()))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn windows_file_is_written_atomically() {
    let scratch = Scratch::new("atomic");
    let keys = new_keys();
    let store = storage(&keys, &scratch.0, "com.example.app", "nsh_secure_storage");
    write(store, "token", "before").unwrap();
    let file = values_files(&scratch.0)
        .pop()
        .expect("the values file exists");
    let temporary = file.with_file_name(format!(
        "{}.tmp",
        file.file_name().unwrap().to_string_lossy()
    ));
    let before = fs::read(&file).unwrap();

    // A folder where the temporary file belongs makes the write fail before the rename.
    fs::create_dir(&temporary).unwrap();
    assert!(
        write(store, "token", "after").is_err(),
        "the write reports the failure"
    );
    assert_eq!(
        fs::read(&file).unwrap(),
        before,
        "a failed write leaves the old file as it was"
    );
    assert_eq!(read(store, "token").unwrap().as_deref(), Some("before"));

    fs::remove_dir(&temporary).unwrap();
    write(store, "token", "after").unwrap();
    assert_eq!(read(store, "token").unwrap().as_deref(), Some("after"));
    assert!(!temporary.exists(), "no temporary file is left behind");
}

#[test]
fn windows_two_apps_do_not_share_a_target() {
    let scratch = Scratch::new("two-apps");
    let keys = new_keys();
    let one = storage(&keys, &scratch.0, "com.example.one", "nsh_secure_storage");
    let two = storage(&keys, &scratch.0, "com.example.two", "nsh_secure_storage");
    write(one, "token", "secret-one").unwrap();
    assert_eq!(
        read(two, "token").unwrap(),
        None,
        "the other app sees nothing"
    );
    write(two, "token", "secret-two").unwrap();
    assert_eq!(read(one, "token").unwrap().as_deref(), Some("secret-one"));
    assert_eq!(read(two, "token").unwrap().as_deref(), Some("secret-two"));
    assert_eq!(
        values_files(&scratch.0).len(),
        2,
        "each app has its own values file"
    );
    block_on(two.delete_all(&Default::default())).unwrap();
    assert_eq!(
        read(one, "token").unwrap().as_deref(),
        Some("secret-one"),
        "deleting one app's values leaves the other's"
    );
}

#[test]
fn windows_target_name_includes_app_identity() {
    let scratch = Scratch::new("target");
    let keys = new_keys();
    let store = storage(&keys, &scratch.0, "com.example.one", "nsh_secure_storage");
    write(store, "token", "value").unwrap();
    let entry = key_entry(&keys, "com.example.one", "nsh_secure_storage");
    let (service, user) = entry
        .get_specifiers()
        .expect("the key entry has its specifiers");
    assert_eq!(
        service,
        BASE64_URL_SAFE_NO_PAD.encode("com.example.one"),
        "the Credential Manager service carries the app identity"
    );
    assert_eq!(user, BASE64_URL_SAFE_NO_PAD.encode("nsh_secure_storage"));

    let other = storage(&keys, &scratch.0, "com.example.two", "nsh_secure_storage");
    assert_eq!(read(other, "token").unwrap(), None);
    let found = keys
        .search(&HashMap::new())
        .expect("the mock can list its entries");
    assert_eq!(found.len(), 1, "reading creates no key for the other app");
}

#[test]
fn a_tampered_file_fails_its_tag() {
    let scratch = Scratch::new("tamper");
    let keys = new_keys();
    let store = storage(&keys, &scratch.0, "com.example.app", "nsh_secure_storage");
    let secret = "s3cret-payload-7f3a";
    write(store, "token", secret).unwrap();

    let file = values_files(&scratch.0)
        .pop()
        .expect("the values file exists");
    let mut json: serde_json::Value =
        serde_json::from_slice(&fs::read(&file).unwrap()).expect("the file is JSON");
    let record = json["records"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .next()
        .expect("one record");
    let mut ciphertext = BASE64_STANDARD
        .decode(record["ciphertext"].as_str().unwrap())
        .unwrap();
    ciphertext[0] ^= 0x01;
    record["ciphertext"] = serde_json::Value::String(BASE64_STANDARD.encode(&ciphertext));
    fs::write(&file, serde_json::to_vec(&json).unwrap()).unwrap();

    let error = read(store, "token").expect_err("a changed record must not open");
    assert_eq!(error.code(), "ERR_SECURE_STORAGE_CORRUPT");
    assert_eq!(error.key(), Some("token"), "the error names the key");
    assert!(
        !error.to_string().contains(secret),
        "the error never shows the value"
    );
}

#[test]
fn a_failed_credential_lookup_never_replaces_the_key() {
    let scratch = Scratch::new("no-replace");
    let keys = new_keys();
    let store = storage(&keys, &scratch.0, "com.example.app", "nsh_secure_storage");
    write(store, "token", "before").unwrap();

    let entry = key_entry(&keys, "com.example.app", "nsh_secure_storage");
    let cred: &mock::Cred = entry.as_any().downcast_ref().expect("a mock credential");
    let key_before = entry.get_secret().unwrap();

    // The Credential Manager lookup fails once: the write must fail, not make a new key.
    cred.set_error(KeyringError::PlatformFailure(Box::new(
        std::io::Error::other("transient"),
    )));
    let error = write(store, "token", "after").expect_err("a failed lookup is an error");
    assert_eq!(error.code(), "ERR_SECURE_STORAGE_BACKEND");
    assert_eq!(error.key(), Some("token"));
    assert_eq!(
        entry.get_secret().unwrap(),
        key_before,
        "the key is the same one"
    );

    cred.set_error(KeyringError::PlatformFailure(Box::new(
        std::io::Error::other("transient"),
    )));
    assert!(
        read(store, "token").is_err(),
        "a failed lookup on read is an error"
    );
    assert_eq!(
        read(store, "token").unwrap().as_deref(),
        Some("before"),
        "the value the old key protects is still readable"
    );
}

#[test]
fn long_values_round_trip() {
    let scratch = Scratch::new("long");
    let keys = new_keys();
    let store = storage(&keys, &scratch.0, "com.example.app", "nsh_secure_storage");
    let value: String = (0..(1 << 20))
        .map(|i| char::from(b'a' + (i % 26) as u8))
        .collect();
    write(store, "big", &value).unwrap();
    assert_eq!(read(store, "big").unwrap().as_deref(), Some(value.as_str()));
}

#[test]
fn reserved_key_characters_round_trip() {
    let scratch = Scratch::new("keys");
    let keys = new_keys();
    let store = storage(&keys, &scratch.0, "com.example.app", "nsh_secure_storage");
    let long = "k".repeat(300);
    let names = [
        "colon:name",
        "slash/name",
        "back\\slash",
        "..",
        "../../escape",
        "ünïcode",
        "emoji 🔑",
        "with\nnewline",
        long.as_str(),
    ];
    for name in names {
        write(store, name, name).unwrap();
    }
    for name in names {
        assert_eq!(read(store, name).unwrap().as_deref(), Some(name));
    }
    assert_eq!(values_files(&scratch.0).len(), 1, "one file for the store");
}

#[test]
fn windows_absent_key_is_none_and_contains_key_agrees() {
    let scratch = Scratch::new("absent");
    let keys = new_keys();
    let store = storage(&keys, &scratch.0, "com.example.app", "nsh_secure_storage");
    assert!(!block_on(store.contains_key("k", &Default::default())).unwrap());
    write(store, "k", "v").unwrap();
    assert!(block_on(store.contains_key("k", &Default::default())).unwrap());
    block_on(store.delete("k", &Default::default())).unwrap();
    assert!(!block_on(store.contains_key("k", &Default::default())).unwrap());
    assert_eq!(read(store, "k").unwrap(), None);
}
