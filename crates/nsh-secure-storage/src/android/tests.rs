//! The Android store logic on the host, against a fake Keystore that seals with real AES-GCM.
//! Assertions on values use `assert!` with no message: a failed test must not print a value.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    future::Future,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicUsize, Ordering},
    },
};

use aes_gcm::{
    Aes256Gcm,
    aead::{Aead, KeyInit, Payload},
};
use zeroize::Zeroizing;

use super::{
    AndroidBackend, file,
    keystore::{Keystore, Policy, Sealed, VaultError},
};
use crate::{
    AndroidOptions, ReadOptions, SecureStorage, SecureStorageError, SecureStorageOptions,
    WriteOptions, options::PlatformOptions,
};

/// The state of the fake Keystore, and the faults a test sets on it.
#[derive(Default)]
struct State {
    keys: BTreeMap<String, [u8; 32]>,
    /// Aliases whose key is permanently invalid.
    invalid: BTreeSet<String>,
    /// Reports no key for any alias, as a null Keystore lookup does.
    hidden: bool,
    fail_has_key: Option<VaultError>,
    fail_seal: Option<VaultError>,
    fail_open: Option<VaultError>,
    /// A key made from now on is invalid at once.
    invalid_on_create: bool,
    fail_dir: bool,
    panic_on_seal: bool,
    created: usize,
    deleted: usize,
    nonce: u64,
}

/// The fake Keystore. It keeps its keys in memory and seals with AES-256-GCM, so a wrong key or
/// a moved entry fails its tag exactly as on a device.
struct Fake {
    dir: PathBuf,
    state: Mutex<State>,
}

impl Fake {
    fn new(dir: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            dir,
            state: Mutex::new(State::default()),
        })
    }

    fn with<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
        let mut state: MutexGuard<'_, State> = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        f(&mut state)
    }
}

/// A 12-byte IV from a counter. A device chooses a random one; the fake only needs them unique.
fn iv_for(counter: u64) -> [u8; 12] {
    let mut iv = [0_u8; 12];
    iv[4..].copy_from_slice(&counter.to_be_bytes());
    iv
}

impl Keystore for Fake {
    fn no_backup_dir(&self) -> Result<PathBuf, VaultError> {
        self.with(|s| {
            if s.fail_dir {
                Err(VaultError::Failed)
            } else {
                Ok(self.dir.clone())
            }
        })
    }

    fn has_key(&self, policy: &Policy) -> Result<bool, VaultError> {
        let alias = policy.alias();
        self.with(|s| {
            if let Some(error) = s.fail_has_key {
                return Err(error);
            }
            Ok(!s.hidden && s.keys.contains_key(&alias))
        })
    }

    fn create_key(&self, policy: &Policy) -> Result<(), VaultError> {
        if policy.strongbox {
            // The test device has no StrongBox.
            return Err(VaultError::NoStrongBox);
        }
        let alias = policy.alias();
        self.with(|s| {
            s.created += 1;
            // A distinct key per creation, so a replaced key cannot open an old entry.
            let fill = u8::try_from(s.created).unwrap_or(u8::MAX);
            s.keys.insert(alias.clone(), [fill; 32]);
            s.invalid.remove(&alias);
            if s.invalid_on_create {
                s.invalid.insert(alias);
            }
            Ok(())
        })
    }

    fn delete_key(&self, policy: &Policy) -> Result<(), VaultError> {
        let alias = policy.alias();
        self.with(|s| {
            if s.keys.remove(&alias).is_some() {
                s.deleted += 1;
            }
            s.invalid.remove(&alias);
            Ok(())
        })
    }

    fn seal(&self, policy: &Policy, aad: &[u8], plaintext: &[u8]) -> Result<Sealed, VaultError> {
        let alias = policy.alias();
        let (key, iv) = self.with(|s| {
            if s.panic_on_seal {
                panic!("the fake keystore panics on purpose");
            }
            if let Some(error) = s.fail_seal {
                return Err(error);
            }
            if s.invalid.contains(&alias) {
                return Err(VaultError::Invalidated);
            }
            let key = *s.keys.get(&alias).ok_or(VaultError::Failed)?;
            s.nonce += 1;
            Ok((key, iv_for(s.nonce)))
        })?;
        let data = Aes256Gcm::new(&key.into())
            .encrypt(
                &iv.into(),
                Payload {
                    msg: plaintext,
                    aad,
                },
            )
            .map_err(|_| VaultError::Failed)?;
        Ok(Sealed {
            iv: iv.to_vec(),
            data,
        })
    }

    fn open(
        &self,
        policy: &Policy,
        aad: &[u8],
        sealed: &Sealed,
    ) -> Result<Zeroizing<Vec<u8>>, VaultError> {
        let alias = policy.alias();
        let key = self.with(|s| {
            if let Some(error) = s.fail_open {
                return Err(error);
            }
            if s.invalid.contains(&alias) {
                return Err(VaultError::Invalidated);
            }
            s.keys.get(&alias).copied().ok_or(VaultError::Failed)
        })?;
        let iv: [u8; 12] = sealed
            .iv
            .as_slice()
            .try_into()
            .map_err(|_| VaultError::Tag)?;
        let plain = Aes256Gcm::new(&key.into())
            .decrypt(
                &iv.into(),
                Payload {
                    msg: &sealed.data,
                    aad,
                },
            )
            .map_err(|_| VaultError::Tag)?;
        Ok(Zeroizing::new(plain))
    }
}

/// One test's Keystore, folder and store handles. The folder is removed when the test ends.
struct Harness {
    fake: Arc<Fake>,
    dir: PathBuf,
    reset_on_error: bool,
}

static NEXT: AtomicUsize = AtomicUsize::new(0);

impl Harness {
    fn new(reset_on_error: bool) -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("nsh-android-test-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a folder for the test");
        Self {
            fake: Fake::new(dir.clone()),
            dir,
            reset_on_error,
        }
    }

    /// A handle on the store named `namespace` (the default store when `None`).
    fn store(&self, namespace: Option<&str>) -> SecureStorage {
        let android = AndroidOptions {
            reset_on_error: self.reset_on_error,
            storage_namespace: namespace.map(str::to_owned),
            ..AndroidOptions::default()
        };
        let platform = PlatformOptions {
            android,
            ..PlatformOptions::default()
        };
        SecureStorage::with_backend(
            Arc::new(AndroidBackend::with_keystore(self.fake.clone())),
            SecureStorageOptions { platform },
        )
    }

    /// The default store.
    fn s(&self) -> SecureStorage {
        self.store(None)
    }

    /// The default store's file.
    fn file(&self) -> PathBuf {
        file::path(&self.dir, "nsh_secure_storage")
    }

    /// The default store's key alias.
    fn alias(&self) -> String {
        Policy::from_options(&PlatformOptions::default()).alias()
    }

    /// The default store's key becomes permanently invalid.
    fn invalidate(&self) {
        let alias = self.alias();
        self.fake.with(|s| {
            s.invalid.insert(alias);
        });
    }

    /// (keys created, keys deleted) so far.
    fn counts(&self) -> (usize, usize) {
        self.fake.with(|s| (s.created, s.deleted))
    }

    /// How many entries the default store's file holds.
    fn entries(&self) -> usize {
        file::load(&self.file()).expect("the file loads").len()
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Runs one future on a runtime, so `nsh_core::blocking` has its pool.
fn block_on<T>(future: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime for the blocking pool")
        .block_on(future)
}

fn put(store: SecureStorage, key: &str, value: &str) -> Result<(), SecureStorageError> {
    block_on(store.write(key, Some(value), &WriteOptions::default()))
}

fn get(store: SecureStorage, key: &str) -> Result<Option<String>, SecureStorageError> {
    block_on(store.read(key, &ReadOptions::default()))
        .map(|value| value.map(|value| value.as_str().to_owned()))
}

/// Whether `key` reads as `expected`. A failed read is a test failure, with no value in it.
fn holds(store: SecureStorage, key: &str, expected: Option<&str>) -> bool {
    get(store, key).expect("the read succeeds").as_deref() == expected
}

fn code(error: &SecureStorageError) -> &'static str {
    error.code()
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

/// Every file under `dir`, at any depth.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).expect("the folder reads").flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(files_under(&path));
        } else {
            out.push(path);
        }
    }
    out
}

#[test]
fn a_transient_keystore_error_never_replaces_the_key() {
    // Reset is on: a transient error must still never reach it.
    let h = Harness::new(true);
    put(h.s(), "token", "first").unwrap();
    h.fake.with(|s| s.fail_seal = Some(VaultError::Failed));
    assert_eq!(
        code(&put(h.s(), "token", "second").unwrap_err()),
        "ERR_SECURE_STORAGE_BACKEND"
    );
    h.fake.with(|s| {
        s.fail_seal = None;
        s.fail_has_key = Some(VaultError::Failed);
    });
    // A failed lookup is not "no key": nothing is created.
    assert_eq!(
        code(&put(h.s(), "other", "x").unwrap_err()),
        "ERR_SECURE_STORAGE_BACKEND"
    );
    h.fake.with(|s| s.fail_has_key = None);
    assert_eq!(h.counts(), (1, 0));
    assert!(holds(h.s(), "token", Some("first")));
}

#[test]
fn a_locked_keystore_is_retryable() {
    let h = Harness::new(true);
    put(h.s(), "token", "first").unwrap();
    h.fake.with(|s| s.fail_open = Some(VaultError::Locked));
    let error = get(h.s(), "token").unwrap_err();
    assert_eq!(code(&error), "ERR_SECURE_STORAGE_LOCKED");
    assert_eq!(error.key(), Some("token"));
    assert_eq!(h.counts(), (1, 0));
    h.fake.with(|s| s.fail_open = None);
    assert!(holds(h.s(), "token", Some("first")));
    assert_eq!(h.entries(), 1);
}

#[test]
fn user_not_authenticated_is_locked_not_mismatch() {
    let h = Harness::new(true);
    put(h.s(), "token", "first").unwrap();
    h.fake.with(|s| s.fail_seal = Some(VaultError::Locked));
    assert_eq!(
        code(&put(h.s(), "token", "second").unwrap_err()),
        "ERR_SECURE_STORAGE_LOCKED"
    );
    // Reset is on, yet a locked keystore never reaches the reset path.
    assert_eq!(h.counts(), (1, 0));
    assert_eq!(h.entries(), 1);
}

#[test]
fn an_invalidated_key_has_a_code_and_keeps_the_file() {
    let h = Harness::new(false);
    put(h.s(), "token", "first").unwrap();
    h.invalidate();
    let error = get(h.s(), "token").unwrap_err();
    assert_eq!(code(&error), "ERR_SECURE_STORAGE_KEY_INVALIDATED");
    assert_eq!(error.key(), Some("token"));
    let error = put(h.s(), "token", "second").unwrap_err();
    assert_eq!(code(&error), "ERR_SECURE_STORAGE_KEY_INVALIDATED");
    assert!(h.file().exists());
    assert_eq!(h.entries(), 1);
    assert_eq!(h.counts(), (1, 0));
}

#[test]
fn reset_on_error_recovers_once_when_opted_in() {
    let h = Harness::new(true);
    put(h.s(), "token", "first").unwrap();
    h.invalidate();
    put(h.s(), "token", "second").unwrap();
    assert!(holds(h.s(), "token", Some("second")));
    assert_eq!(h.counts(), (2, 1));
    assert_eq!(h.entries(), 1);
}

#[test]
fn reset_on_error_runs_at_most_once() {
    let h = Harness::new(true);
    put(h.s(), "token", "first").unwrap();
    // The fresh key is invalid too: one reset is tried, then the error is reported.
    h.fake.with(|s| s.invalid_on_create = true);
    h.invalidate();
    let error = put(h.s(), "token", "second").unwrap_err();
    // R3: a fresh key that fails too is reset_failed, reported once.
    assert_eq!(code(&error), "ERR_SECURE_STORAGE_RESET_FAILED");
    assert_eq!(h.counts(), (2, 1));
}

#[test]
fn reset_on_error_defaults_to_false() {
    assert!(!AndroidOptions::default().reset_on_error);
    assert!(!Policy::from_options(&PlatformOptions::default()).reset_on_error);
}

#[test]
fn android_null_keystore_read_with_stored_data_keeps_key() {
    let h = Harness::new(true);
    put(h.s(), "token", "first").unwrap();
    h.fake.with(|s| s.hidden = true);
    assert_eq!(
        code(&get(h.s(), "token").unwrap_err()),
        "ERR_SECURE_STORAGE_KEY_MISSING"
    );
    assert_eq!(
        code(&put(h.s(), "token", "second").unwrap_err()),
        "ERR_SECURE_STORAGE_KEY_MISSING"
    );
    h.fake.with(|s| s.hidden = false);
    assert_eq!(h.counts(), (1, 0));
    assert!(holds(h.s(), "token", Some("first")));
}

#[test]
fn restored_ciphertext_without_key_is_kept_and_reported() {
    let h = Harness::new(true);
    put(h.s(), "token", "first").unwrap();
    // A restore brings the file back without the Keystore key.
    h.fake.with(|s| s.keys.clear());
    let error = get(h.s(), "token").unwrap_err();
    assert_eq!(code(&error), "ERR_SECURE_STORAGE_KEY_MISSING");
    assert_eq!(error.key(), Some("token"));
    assert_eq!(
        code(&put(h.s(), "token", "second").unwrap_err()),
        "ERR_SECURE_STORAGE_KEY_MISSING"
    );
    assert!(h.file().exists());
    assert_eq!(h.entries(), 1);
    assert_eq!(h.counts(), (1, 0));
}

#[test]
fn ciphertext_moved_to_another_key_fails_its_tag() {
    // Reset is off: a failed tag is reported and kept, not wiped.
    let h = Harness::new(false);
    put(h.s(), "one", "secret-one").unwrap();
    let mut entries = file::load(&h.file()).unwrap();
    let moved = entries["one"].clone();
    entries.insert("two".to_owned(), moved);
    file::save(&h.file(), &entries).unwrap();
    let error = get(h.s(), "two").unwrap_err();
    assert_eq!(code(&error), "ERR_SECURE_STORAGE_CORRUPT");
    assert_eq!(error.key(), Some("two"));
    assert!(holds(h.s(), "one", Some("secret-one")));
    assert_eq!(h.counts(), (1, 0));
}

#[test]
fn file_written_atomically() {
    let h = Harness::new(false);
    put(h.s(), "token", "first").unwrap();
    let before = fs::read(h.file()).unwrap();
    // A write that fails while sealing leaves the old file whole.
    h.fake.with(|s| s.fail_seal = Some(VaultError::Failed));
    assert_eq!(
        code(&put(h.s(), "token", "second").unwrap_err()),
        "ERR_SECURE_STORAGE_BACKEND"
    );
    assert!(fs::read(h.file()).unwrap() == before);
    h.fake.with(|s| s.fail_seal = None);
    put(h.s(), "token", "second").unwrap();
    assert!(holds(h.s(), "token", Some("second")));
    // No temporary file is left, and the file on disk holds no plaintext.
    assert!(!h.file().with_extension("json.tmp").exists());
    let on_disk = fs::read(h.file()).unwrap();
    assert!(!contains(&on_disk, "second"));
    assert!(!contains(&on_disk, "first"));
}

#[test]
fn delete_all_touches_only_its_own_store() {
    let h = Harness::new(false);
    put(h.store(Some("a")), "k", "from-a").unwrap();
    put(h.store(Some("b")), "k", "from-b").unwrap();
    block_on(h.store(Some("a")).delete_all(&Default::default())).unwrap();
    assert!(holds(h.store(Some("a")), "k", None));
    assert!(holds(h.store(Some("b")), "k", Some("from-b")));
    let alias_b = Policy::from_options(&PlatformOptions {
        android: AndroidOptions {
            storage_namespace: Some("b".to_owned()),
            ..AndroidOptions::default()
        },
        ..PlatformOptions::default()
    })
    .alias();
    assert!(h.fake.with(|s| s.keys.contains_key(&alias_b)));
}

#[test]
fn store_names_isolate_data() {
    let h = Harness::new(false);
    put(h.store(Some("a")), "k", "from-a").unwrap();
    assert!(holds(h.store(Some("c")), "k", None));
    assert!(holds(h.s(), "k", None));
}

#[test]
fn reserved_key_characters_round_trip() {
    let h = Harness::new(false);
    let long = "k".repeat(300);
    let keys: [&str; 7] = ["a:b", "a/b", "a\\b", "..", "é漢", "🔐", long.as_str()];
    for (i, key) in keys.iter().enumerate() {
        put(h.s(), key, &format!("value-{i}")).unwrap();
    }
    for (i, key) in keys.iter().enumerate() {
        assert!(holds(h.s(), key, Some(&format!("value-{i}"))));
    }
    let all = block_on(h.s().read_all(&Default::default())).unwrap();
    assert_eq!(all.len(), keys.len());
    // Keys never become file names: the folder holds one store file, inside its own folder.
    let files = files_under(&h.dir);
    assert_eq!(files.len(), 1);
    assert!(files[0].starts_with(h.dir.join("nsh-secure-storage")));
}

#[test]
fn large_value_round_trips() {
    let h = Harness::new(false);
    let big = "x".repeat(1 << 20);
    put(h.s(), "big", &big).unwrap();
    assert!(holds(h.s(), "big", Some(&big)));
}

#[test]
fn backend_load_failure_is_coded_error() {
    let h = Harness::new(true);
    h.fake.with(|s| s.fail_dir = true);
    let error = get(h.s(), "token").unwrap_err();
    assert_eq!(code(&error), "ERR_SECURE_STORAGE_BACKEND");
    h.fake.with(|s| s.fail_dir = false);
    put(h.s(), "token", "first").unwrap();
    // A panic in the Keystore is a coded error too, and the store stays usable.
    h.fake.with(|s| s.panic_on_seal = true);
    assert_eq!(
        code(&put(h.s(), "token", "second").unwrap_err()),
        "ERR_SECURE_STORAGE_BACKEND"
    );
    h.fake.with(|s| s.panic_on_seal = false);
    assert!(holds(h.s(), "token", Some("first")));
}

#[test]
fn error_text_never_contains_the_value() {
    let h = Harness::new(false);
    put(h.s(), "token", "SECRET-VALUE-4242").unwrap();
    h.invalidate();
    let error = get(h.s(), "token").unwrap_err();
    let text = format!("{error} {error:?}");
    assert!(!text.contains("SECRET-VALUE-4242"));
    assert!(text.contains("token"));
}

#[test]
fn require_strong_box_without_strongbox_is_not_available() {
    let h = Harness::new(false);
    let android = AndroidOptions {
        require_strong_box: true,
        ..AndroidOptions::default()
    };
    let store = SecureStorage::with_backend(
        Arc::new(AndroidBackend::with_keystore(h.fake.clone())),
        SecureStorageOptions {
            platform: PlatformOptions {
                android,
                ..PlatformOptions::default()
            },
        },
    );
    let error = put(store, "token", "first").unwrap_err();
    assert_eq!(code(&error), "ERR_SECURE_STORAGE_NOT_AVAILABLE");
    // No key and no file are made when the device cannot give the key.
    assert_eq!(h.counts(), (0, 0));
    assert!(!h.file().exists());
}
