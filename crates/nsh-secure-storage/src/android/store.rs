//! The store's operations over a [`Keystore`]. A transient failure never changes the key or the
//! file. Only a permanent signal may delete, and only when the policy opts in with `reset_on_error`.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use zeroize::Zeroizing;

use super::{
    file,
    keystore::{Keystore, Policy, Sealed, VaultError},
};
use crate::error::{ErrorKind, SecureStorageError};

/// The additional data every entry is sealed with: its store and its key, length-prefixed. An
/// entry moved to another key or another store fails its tag.
fn aad(namespace: &str, key: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + namespace.len() + key.len());
    out.extend_from_slice(&(namespace.len() as u32).to_be_bytes());
    out.extend_from_slice(namespace.as_bytes());
    out.extend_from_slice(&(key.len() as u32).to_be_bytes());
    out.extend_from_slice(key.as_bytes());
    out
}

/// What opening one entry gave: its value, or a permanent failure that the policy may reset.
enum Opened {
    Value(Zeroizing<String>),
    Reset,
}

fn file_of(keystore: &dyn Keystore, policy: &Policy) -> Result<PathBuf, SecureStorageError> {
    let root = keystore.no_backup_dir().map_err(VaultError::error)?;
    Ok(file::path(&root, policy.namespace()))
}

/// Whether the key exists. A failed lookup is an error, never "absent", so it cannot lead to a new key.
fn key_exists(
    keystore: &dyn Keystore,
    policy: &Policy,
    key: &str,
) -> Result<bool, SecureStorageError> {
    keystore
        .has_key(policy)
        .map_err(|error| error.error().with_key(key))
}

/// Makes the store's key. Called only when none exists, or after a reset removed it.
fn create_key(
    keystore: &dyn Keystore,
    policy: &Policy,
    key: &str,
) -> Result<(), SecureStorageError> {
    keystore
        .create_key(policy)
        .map_err(|error| error.error().with_key(key))
}

/// Removes the store's file, then its key: the data goes first, so a failure after it leaves
/// only a key with nothing sealed under it.
fn reset(
    keystore: &dyn Keystore,
    policy: &Policy,
    path: &Path,
    key: &str,
) -> Result<(), SecureStorageError> {
    file::remove(path).map_err(|error| error.with_key(key))?;
    keystore
        .delete_key(policy)
        .map_err(|error| error.error().with_key(key))
}

/// Opens one sealed entry. A key that is missing while data exists is reported and kept.
fn open_entry(
    keystore: &dyn Keystore,
    policy: &Policy,
    key: &str,
    sealed: &Sealed,
) -> Result<Opened, SecureStorageError> {
    if !key_exists(keystore, policy, key)? {
        return Err(VaultError::Invalidated.error().with_key(key));
    }
    match keystore.open(policy, &aad(policy.namespace(), key), sealed) {
        Ok(bytes) => {
            let text = std::str::from_utf8(&bytes)
                .map_err(|_| SecureStorageError::new(ErrorKind::Corrupt).with_key(key))?;
            Ok(Opened::Value(Zeroizing::new(text.to_owned())))
        }
        Err(VaultError::Invalidated | VaultError::Tag) if policy.reset_on_error => {
            Ok(Opened::Reset)
        }
        Err(error) => Err(error.error().with_key(key)),
    }
}

/// Writes `value` under `key`. A new key is made only when the store has no key and no data.
pub(crate) fn write(
    keystore: &dyn Keystore,
    policy: &Policy,
    key: &str,
    value: &str,
) -> Result<(), SecureStorageError> {
    let path = file_of(keystore, policy)?;
    let mut entries = file::load(&path).map_err(|error| error.with_key(key))?;
    if !key_exists(keystore, policy, key)? {
        if !entries.is_empty() {
            return Err(VaultError::Invalidated.error().with_key(key));
        }
        create_key(keystore, policy, key)?;
    }
    let aad = aad(policy.namespace(), key);
    let sealed = match keystore.seal(policy, &aad, value.as_bytes()) {
        Ok(sealed) => sealed,
        Err(VaultError::Invalidated) if policy.reset_on_error => {
            // The one retry. A second failure is reported, and nothing else is tried.
            reset(keystore, policy, &path, key)?;
            entries.clear();
            create_key(keystore, policy, key)?;
            keystore
                .seal(policy, &aad, value.as_bytes())
                .map_err(|error| error.error().with_key(key))?
        }
        Err(error) => return Err(error.error().with_key(key)),
    };
    entries.insert(key.to_owned(), sealed);
    file::save(&path, &entries).map_err(|error| error.with_key(key))
}

/// The value under `key`, or `None` when the store has no entry for it.
pub(crate) fn read(
    keystore: &dyn Keystore,
    policy: &Policy,
    key: &str,
) -> Result<Option<Zeroizing<String>>, SecureStorageError> {
    let path = file_of(keystore, policy)?;
    let entries = file::load(&path).map_err(|error| error.with_key(key))?;
    let Some(sealed) = entries.get(key) else {
        return Ok(None);
    };
    match open_entry(keystore, policy, key, sealed)? {
        Opened::Value(value) => Ok(Some(value)),
        Opened::Reset => {
            reset(keystore, policy, &path, key)?;
            Ok(None)
        }
    }
}

/// Whether the store has an entry for `key`. It does not open the entry, so it never prompts.
pub(crate) fn contains_key(
    keystore: &dyn Keystore,
    policy: &Policy,
    key: &str,
) -> Result<bool, SecureStorageError> {
    let path = file_of(keystore, policy)?;
    let entries = file::load(&path).map_err(|error| error.with_key(key))?;
    Ok(entries.contains_key(key))
}

/// Removes the entry for `key`. A key that is not there is not an error.
pub(crate) fn delete(
    keystore: &dyn Keystore,
    policy: &Policy,
    key: &str,
) -> Result<(), SecureStorageError> {
    let path = file_of(keystore, policy)?;
    let mut entries = file::load(&path).map_err(|error| error.with_key(key))?;
    if entries.remove(key).is_some() {
        file::save(&path, &entries).map_err(|error| error.with_key(key))?;
    }
    Ok(())
}

/// Every entry of the store, in key order.
pub(crate) fn read_all(
    keystore: &dyn Keystore,
    policy: &Policy,
) -> Result<BTreeMap<String, Zeroizing<String>>, SecureStorageError> {
    let path = file_of(keystore, policy)?;
    let entries = file::load(&path)?;
    let mut all = BTreeMap::new();
    for (key, sealed) in &entries {
        match open_entry(keystore, policy, key, sealed)? {
            Opened::Value(value) => {
                all.insert(key.clone(), value);
            }
            Opened::Reset => {
                reset(keystore, policy, &path, key)?;
                return Ok(BTreeMap::new());
            }
        }
    }
    Ok(all)
}

/// Removes every entry of this store and its key, and nothing of any other store. The next write
/// makes a fresh key, which is also how a store whose key was invalidated gets back to use.
pub(crate) fn delete_all(
    keystore: &dyn Keystore,
    policy: &Policy,
) -> Result<(), SecureStorageError> {
    let path = file_of(keystore, policy)?;
    file::remove(&path)?;
    keystore.delete_key(policy).map_err(VaultError::error)
}
