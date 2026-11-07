//! The store's file: one JSON file per store, in `no_backup/nsh-secure-storage/`. Keys are
//! encoded, so any key name is a safe entry; the values are sealed, so the file holds no plaintext.

use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use base64::prelude::{BASE64_STANDARD, BASE64_URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};

use super::keystore::Sealed;
use crate::error::{ErrorKind, SecureStorageError};

/// The folder under the no-backup directory that holds every store file.
const DIR: &str = "nsh-secure-storage";
/// The file format version. A file of another version is reported as corrupt, never rewritten.
const VERSION: u32 = 1;

/// The sealed entries of one store, by key.
pub(crate) type Entries = BTreeMap<String, Sealed>;

/// The file of the store named `namespace`, under the no-backup directory `root`.
pub(crate) fn path(root: &Path, namespace: &str) -> PathBuf {
    root.join(DIR)
        .join(format!("{}.json", BASE64_URL_SAFE_NO_PAD.encode(namespace)))
}

#[derive(Serialize, Deserialize)]
struct Wire {
    version: u32,
    entries: BTreeMap<String, WireEntry>,
}

#[derive(Serialize, Deserialize)]
struct WireEntry {
    iv: String,
    data: String,
}

/// The entries in `path`, or none when the file does not exist. A file that does not parse is
/// `Corrupt`, and it is kept.
pub(crate) fn load(path: &Path) -> Result<Entries, SecureStorageError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Entries::new()),
        Err(_) => return Err(SecureStorageError::new(ErrorKind::Backend)),
    };
    let corrupt = || SecureStorageError::new(ErrorKind::Corrupt);
    let wire: Wire = serde_json::from_slice(&bytes).map_err(|_| corrupt())?;
    if wire.version != VERSION {
        return Err(corrupt());
    }
    wire.entries
        .into_iter()
        .map(|(encoded, entry)| {
            let key = BASE64_URL_SAFE_NO_PAD
                .decode(encoded)
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .ok_or_else(corrupt)?;
            let iv = BASE64_STANDARD.decode(entry.iv).map_err(|_| corrupt())?;
            let data = BASE64_STANDARD.decode(entry.data).map_err(|_| corrupt())?;
            Ok((key, Sealed { iv, data }))
        })
        .collect()
}

/// Writes `entries` to `path` in one step: a temporary file is written, synced and renamed over
/// the old file, so a failure leaves the old file whole. No entries removes the file.
pub(crate) fn save(path: &Path, entries: &Entries) -> Result<(), SecureStorageError> {
    if entries.is_empty() {
        return remove(path);
    }
    let io = || SecureStorageError::new(ErrorKind::Backend);
    let dir = path.parent().ok_or_else(io)?;
    fs::create_dir_all(dir).map_err(|_| io())?;
    let wire = Wire {
        version: VERSION,
        entries: entries
            .iter()
            .map(|(key, sealed)| {
                let entry = WireEntry {
                    iv: BASE64_STANDARD.encode(&sealed.iv),
                    data: BASE64_STANDARD.encode(&sealed.data),
                };
                (BASE64_URL_SAFE_NO_PAD.encode(key), entry)
            })
            .collect(),
    };
    let bytes = serde_json::to_vec(&wire).map_err(|_| io())?;
    let temporary = path.with_extension("json.tmp");
    let written = File::create(&temporary).and_then(|mut file| {
        file.write_all(&bytes)?;
        file.sync_all()
    });
    if written.is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(io());
    }
    fs::rename(&temporary, path).map_err(|_| {
        let _ = fs::remove_file(&temporary);
        io()
    })
}

/// Removes the file at `path`. A missing file is not an error.
pub(crate) fn remove(path: &Path) -> Result<(), SecureStorageError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(SecureStorageError::new(ErrorKind::Backend)),
    }
}
