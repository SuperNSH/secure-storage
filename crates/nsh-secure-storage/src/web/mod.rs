//! The web backend. Each value is sealed with AES-256-GCM under one non-extractable WebCrypto key
//! per store, and both are kept in IndexedDB. Nothing is written to `localStorage`.
//!
//! The browser has no secret store. A script on the page can use the key, though it cannot
//! export it, so this protects the stored files, not the page. See the "Web backend" section of `docs/decisions.md`.

mod crypto;
mod db;

use std::collections::BTreeMap;

use zeroize::Zeroizing;

use crate::{
    backend::{Backend, BackendFuture},
    error::{ErrorKind, SecureStorageError},
    options::PlatformOptions,
};

/// A failure of the browser's own storage or crypto. Its details never reach an error.
fn backend_error() -> SecureStorageError {
    SecureStorageError::new(ErrorKind::Backend)
}

/// The browser offers no storage or crypto here (a worker, or a browser without them).
fn unavailable() -> SecureStorageError {
    SecureStorageError::new(ErrorKind::NotAvailable)
}

/// The web backend. It holds no browser objects, so it is `Send` and `Sync` like the others.
pub struct WebBackend;

/// Checks what every call needs before it touches IndexedDB, and names the store.
fn prepare(options: &PlatformOptions) -> Result<&str, SecureStorageError> {
    // A key in IndexedDB outlives the tab, so "this tab only" cannot be honoured. Refuse, never fall back.
    if options.web.use_session_storage {
        return Err(SecureStorageError::new(ErrorKind::NotAvailable));
    }
    crypto::require_secure_context()?;
    Ok(&options.web.db_name)
}

impl Backend for WebBackend {
    fn store_id(&self, options: &PlatformOptions) -> String {
        format!("web:{}", options.web.db_name)
    }

    fn write<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
        value: &'a str,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        Box::pin(async move {
            let store = prepare(options)?;
            let db = db::Db::open(store).await?;
            let cipher_key = crypto::load_or_create_key(&db, store).await?;
            let record =
                crypto::seal(&cipher_key, &crypto::aad(store, key), value.as_bytes()).await?;
            db.put_entry(store, key, &record).await
        })
    }

    fn read<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<Option<Zeroizing<String>>, SecureStorageError>> {
        Box::pin(async move {
            let store = prepare(options)?;
            let db = db::Db::open(store).await?;
            read_one(&db, store, key).await
        })
    }

    fn contains_key<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<bool, SecureStorageError>> {
        Box::pin(async move {
            let store = prepare(options)?;
            let db = db::Db::open(store).await?;
            // Same answer as `read`: an entry that cannot be decrypted is absent.
            Ok(read_one(&db, store, key).await?.is_some())
        })
    }

    fn delete<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        Box::pin(async move {
            let store = prepare(options)?;
            let db = db::Db::open(store).await?;
            db.delete_entry(store, key).await
        })
    }

    fn read_all<'a>(
        &'a self,
        options: &'a PlatformOptions,
    ) -> BackendFuture<'a, Result<BTreeMap<String, Zeroizing<String>>, SecureStorageError>> {
        Box::pin(async move {
            let store = prepare(options)?;
            let db = db::Db::open(store).await?;
            let entries = db.entries(store).await?;
            let mut values = BTreeMap::new();
            if entries.is_empty() {
                return Ok(values);
            }
            let cipher_key = db.get_key(store).await?;
            for (key, record) in entries {
                let opened = open_record(cipher_key.as_ref(), store, &key, &record).await;
                if let Some(value) = opened {
                    values.insert(key, value);
                }
            }
            Ok(values)
        })
    }

    fn delete_all<'a>(
        &'a self,
        options: &'a PlatformOptions,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        Box::pin(async move {
            let store = prepare(options)?;
            let db = db::Db::open(store).await?;
            db.delete_entries(store).await
        })
    }
}

/// The value under `key`, or `None` when there is none or it cannot be decrypted.
async fn read_one(
    db: &db::Db,
    store: &str,
    key: &str,
) -> Result<Option<Zeroizing<String>>, SecureStorageError> {
    let Some(record) = db.get_entry(store, key).await? else {
        return Ok(None);
    };
    let cipher_key = db.get_key(store).await?;
    Ok(open_record(cipher_key.as_ref(), store, key, &record).await)
}

/// Opens one record. A record that cannot be opened reads as absent, and is logged by its key only.
async fn open_record(
    cipher_key: Option<&web_sys::CryptoKey>,
    store: &str,
    key: &str,
    record: &[u8],
) -> Option<Zeroizing<String>> {
    let opened = match cipher_key {
        Some(cipher_key) => crypto::open(cipher_key, &crypto::aad(store, key), record).await,
        None => None,
    };
    if opened.is_none() {
        crypto::log_undecryptable(key);
    }
    opened
}
