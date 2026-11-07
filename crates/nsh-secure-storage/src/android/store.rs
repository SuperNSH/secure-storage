//! The store's operations over a [`Keystore`]. A transient failure never changes the key or the
//! file. The core owns the reset rule: this file reports a permanent failure as `KeyInvalidated`,
//! and [`reset`] removes the store when the core asks for it.

use std::{collections::BTreeMap, path::PathBuf};

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

/// Makes the store's key. Called only when none exists.
fn create_key(
    keystore: &dyn Keystore,
    policy: &Policy,
    key: &str,
) -> Result<(), SecureStorageError> {
    keystore
        .create_key(policy)
        .map_err(|error| error.error().with_key(key))
}

/// Opens one sealed entry. Data whose key is gone is reported as `KeyMissing` and kept: a key
/// that cannot be found is not a permanent signal, so the core never resets for it.
fn open_entry(
    keystore: &dyn Keystore,
    policy: &Policy,
    key: &str,
    sealed: &Sealed,
) -> Result<Zeroizing<String>, SecureStorageError> {
    if !key_exists(keystore, policy, key)? {
        return Err(SecureStorageError::new(ErrorKind::KeyMissing).with_key(key));
    }
    let bytes = keystore
        .open(policy, &aad(policy.namespace(), key), sealed)
        .map_err(|error| error.error().with_key(key))?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| SecureStorageError::new(ErrorKind::Corrupt).with_key(key))?;
    Ok(Zeroizing::new(text.to_owned()))
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
            return Err(SecureStorageError::new(ErrorKind::KeyMissing).with_key(key));
        }
        create_key(keystore, policy, key)?;
    }
    let sealed = keystore
        .seal(policy, &aad(policy.namespace(), key), value.as_bytes())
        .map_err(|error| error.error().with_key(key))?;
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
    match entries.get(key) {
        Some(sealed) => open_entry(keystore, policy, key, sealed).map(Some),
        None => Ok(None),
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
        all.insert(key.clone(), open_entry(keystore, policy, key, sealed)?);
    }
    Ok(all)
}

/// Removes the store's file, then its key, and nothing of any other store. The next write makes a
/// fresh key. Used by `delete_all`, and by the backend's `reset_store` when the core asks.
pub(crate) fn reset(keystore: &dyn Keystore, policy: &Policy) -> Result<(), SecureStorageError> {
    let path = file_of(keystore, policy)?;
    file::remove(&path)?;
    keystore.delete_key(policy).map_err(VaultError::error)
}
