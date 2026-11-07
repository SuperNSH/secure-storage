//! The values file of one store: one AES-256-GCM record per key, written whole through a
//! temporary file and a rename. The records hold no key names in clear text.

use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, Write},
    path::Path,
    thread,
    time::Duration,
};

use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use base64::{Engine, prelude::BASE64_STANDARD};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::{
    error::{ErrorKind, SecureStorageError},
    keyring_store::{decode_name, encode_name},
};

/// The file format. A file of another version is `Corrupt`, never guessed at.
const FORMAT_VERSION: u32 = 1;
const NONCE_LEN: usize = 12;
/// Rename attempts before a file that another program holds open is reported as `Locked`.
const RENAME_ATTEMPTS: u32 = 5;

/// One value: its nonce and its ciphertext with the 16-byte tag at the end.
#[derive(Clone)]
pub(super) struct Record {
    nonce: [u8; NONCE_LEN],
    ciphertext: Vec<u8>,
}

/// The records of one store, by key name.
#[derive(Default)]
pub(super) struct ValuesFile {
    records: BTreeMap<String, Record>,
}

#[derive(Serialize, Deserialize)]
struct OnDisk {
    version: u32,
    /// Keyed by the encoded key name.
    records: BTreeMap<String, OnDiskRecord>,
}

#[derive(Serialize, Deserialize)]
struct OnDiskRecord {
    nonce: String,
    ciphertext: String,
}

impl ValuesFile {
    /// The file at `path`, or an empty one when there is none.
    pub(super) fn load(path: &Path) -> Result<Self, SecureStorageError> {
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(io_error(&e)),
        };
        let on_disk: OnDisk = serde_json::from_slice(&bytes).map_err(|_| corrupt())?;
        if on_disk.version != FORMAT_VERSION {
            return Err(corrupt());
        }
        let mut records = BTreeMap::new();
        for (encoded, record) in on_disk.records {
            let key = decode_name(&encoded).ok_or_else(corrupt)?;
            let nonce = BASE64_STANDARD
                .decode(record.nonce)
                .ok()
                .and_then(|bytes| <[u8; NONCE_LEN]>::try_from(bytes).ok())
                .ok_or_else(corrupt)?;
            let ciphertext = BASE64_STANDARD
                .decode(record.ciphertext)
                .map_err(|_| corrupt())?;
            records.insert(key, Record { nonce, ciphertext });
        }
        Ok(Self { records })
    }

    /// Writes the file at `path`, replacing the old one only once the new one is complete.
    pub(super) fn save(&self, path: &Path) -> Result<(), SecureStorageError> {
        let on_disk = OnDisk {
            version: FORMAT_VERSION,
            records: self
                .records
                .iter()
                .map(|(key, record)| {
                    (
                        encode_name(key),
                        OnDiskRecord {
                            nonce: BASE64_STANDARD.encode(record.nonce),
                            ciphertext: BASE64_STANDARD.encode(&record.ciphertext),
                        },
                    )
                })
                .collect(),
        };
        let bytes = serde_json::to_vec(&on_disk)
            .map_err(|_| SecureStorageError::new(ErrorKind::Backend))?;
        write_atomic(path, &bytes)
    }

    pub(super) fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub(super) fn get(&self, key: &str) -> Option<&Record> {
        self.records.get(key)
    }

    pub(super) fn insert(&mut self, key: &str, record: Record) {
        self.records.insert(key.to_owned(), record);
    }

    /// Removes `key`. `true` when there was a record to remove.
    pub(super) fn remove(&mut self, key: &str) -> bool {
        self.records.remove(key).is_some()
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = (&String, &Record)> {
        self.records.iter()
    }
}

/// Encrypts `plaintext` under `key`, binding it to `aad`. A new random nonce for each value.
pub(super) fn seal(
    key: &[u8; 32],
    aad: &[u8],
    plaintext: &[u8],
) -> Result<Record, SecureStorageError> {
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::fill(&mut nonce).map_err(|_| SecureStorageError::new(ErrorKind::Backend))?;
    let ciphertext = cipher(key)
        .encrypt(
            &Nonce::<aes_gcm::aead::consts::U12>::from(nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| SecureStorageError::new(ErrorKind::Backend))?;
    Ok(Record { nonce, ciphertext })
}

/// Decrypts `record` under `key`. A failed tag is `Corrupt`: the value is never returned.
pub(super) fn open(
    key: &[u8; 32],
    aad: &[u8],
    record: &Record,
) -> Result<Zeroizing<String>, SecureStorageError> {
    let plaintext = Zeroizing::new(
        cipher(key)
            .decrypt(
                &Nonce::<aes_gcm::aead::consts::U12>::from(record.nonce),
                Payload {
                    msg: &record.ciphertext,
                    aad,
                },
            )
            .map_err(|_| corrupt())?,
    );
    let text = std::str::from_utf8(&plaintext).map_err(|_| corrupt())?;
    Ok(Zeroizing::new(text.to_owned()))
}

fn cipher(key: &[u8; 32]) -> Aes256Gcm {
    Aes256Gcm::new(key.into())
}

/// Writes `bytes` to `path` through a temporary file beside it. A failed write leaves `path` as it
/// was and removes the temporary file.
pub(super) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), SecureStorageError> {
    let parent = path
        .parent()
        .ok_or_else(|| SecureStorageError::new(ErrorKind::Backend))?;
    fs::create_dir_all(parent).map_err(|e| io_error(&e))?;
    let name = path
        .file_name()
        .ok_or_else(|| SecureStorageError::new(ErrorKind::Backend))?;
    let mut temporary_name = name.to_os_string();
    temporary_name.push(".tmp");
    let temporary = parent.join(temporary_name);

    let written = (|| -> io::Result<()> {
        let mut file = File::create(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()
    })();
    if let Err(e) = written {
        let _ = fs::remove_file(&temporary);
        return Err(io_error(&e));
    }

    let mut attempts = 0;
    loop {
        match fs::rename(&temporary, path) {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied && attempts < RENAME_ATTEMPTS => {
                attempts += 1;
                thread::sleep(Duration::from_millis(20));
            }
            Err(e) => {
                let _ = fs::remove_file(&temporary);
                return Err(io_error(&e));
            }
        }
    }
}

/// Removes the file at `path`. A file that is not there is removed already.
pub(super) fn remove_file(path: &Path) -> Result<(), SecureStorageError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_error(&e)),
    }
}

/// A file the system holds (`PermissionDenied`, a sharing violation on Windows) is `Locked`:
/// retry later. Anything else is a backend failure.
fn io_error(error: &io::Error) -> SecureStorageError {
    let kind = match error.kind() {
        io::ErrorKind::PermissionDenied => ErrorKind::Locked,
        _ => ErrorKind::Backend,
    };
    SecureStorageError::new(kind)
}

fn corrupt() -> SecureStorageError {
    SecureStorageError::new(ErrorKind::Corrupt)
}
