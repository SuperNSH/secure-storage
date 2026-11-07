//! The Linux backend. Each key is one Secret Service item, through `keyring-core`'s store. The
//! item's `service` attribute is the store's name and its `username` is the key.

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

#[cfg(target_os = "linux")]
use std::sync::{Mutex, PoisonError};

use keyring_core::{CredentialStore, Entry, Error as KeyringError};
use zeroize::Zeroizing;

use crate::{
    backend::{Backend, BackendFuture},
    error::{ErrorKind, SecureStorageError},
    keyring_store::map_error,
    options::PlatformOptions,
};

/// The backend for Linux. It reads the Secret Service, or a store it was given.
pub struct LinuxBackend {
    source: Arc<Source>,
}

/// Where the store comes from. The Secret Service is connected on first use, from the blocking
/// pool, so the engine thread never waits on D-Bus.
enum Source {
    Fixed(Arc<CredentialStore>),
    #[cfg(target_os = "linux")]
    SecretService(Mutex<Option<Arc<CredentialStore>>>),
}

impl LinuxBackend {
    /// A backend on `store`. Apps and tests use this to supply a store.
    pub fn with_store(store: Arc<CredentialStore>) -> Self {
        Self {
            source: Arc::new(Source::Fixed(store)),
        }
    }

    /// The backend on the user's Secret Service.
    #[cfg(target_os = "linux")]
    pub fn system() -> Self {
        Self {
            source: Arc::new(Source::SecretService(Mutex::new(None))),
        }
    }
}

impl Source {
    /// The store. A Secret Service that is not running yet is an error now, and is retried later.
    fn store(&self) -> Result<Arc<CredentialStore>, SecureStorageError> {
        match self {
            Source::Fixed(store) => Ok(Arc::clone(store)),
            #[cfg(target_os = "linux")]
            Source::SecretService(cached) => {
                let mut cached = cached.lock().unwrap_or_else(PoisonError::into_inner);
                if let Some(store) = cached.as_ref() {
                    return Ok(Arc::clone(store));
                }
                let store: Arc<CredentialStore> =
                    zbus_secret_service_keyring_store::Store::new().map_err(map_error)?;
                *cached = Some(Arc::clone(&store));
                Ok(store)
            }
        }
    }
}

/// What one call addresses. Owned, so the blocking work can run with it.
struct Call {
    source: Arc<Source>,
    account: String,
}

impl Call {
    fn store(&self) -> Result<Arc<CredentialStore>, SecureStorageError> {
        self.source.store()
    }
}

/// The Secret Service item of `key` in the store named `account`.
fn entry(store: &CredentialStore, account: &str, key: &str) -> Result<Entry, SecureStorageError> {
    store.build(account, key, None).map_err(map_error)
}

/// Writes `value` and reads it back. A store that accepts the write but keeps other bytes is
/// retried once, then reported: a silent miss would look like success.
fn write(call: &Call, key: &str, value: &[u8]) -> Result<(), SecureStorageError> {
    let store = call.store()?;
    let entry = entry(&*store, &call.account, key)?;
    for _ in 0..2 {
        entry
            .set_secret(value)
            .map_err(|e| map_error(e).with_key(key))?;
        let stored = Zeroizing::new(entry.get_secret().map_err(|e| map_error(e).with_key(key))?);
        if stored.as_slice() == value {
            return Ok(());
        }
    }
    Err(SecureStorageError::new(ErrorKind::Backend).with_key(key))
}

fn read(call: &Call, key: &str) -> Result<Option<Zeroizing<String>>, SecureStorageError> {
    let store = call.store()?;
    let entry = entry(&*store, &call.account, key)?;
    match entry.get_secret() {
        Ok(bytes) => decode(Zeroizing::new(bytes))
            .map(Some)
            .map_err(|e| e.with_key(key)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(error) => Err(map_error(error).with_key(key)),
    }
}

fn delete(call: &Call, key: &str) -> Result<(), SecureStorageError> {
    let store = call.store()?;
    let entry = entry(&*store, &call.account, key)?;
    match entry.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(error) => Err(map_error(error).with_key(key)),
    }
}

fn read_all(call: &Call) -> Result<BTreeMap<String, Zeroizing<String>>, SecureStorageError> {
    let store = call.store()?;
    let mut all = BTreeMap::new();
    for (key, item) in items(&*store, &call.account)? {
        let bytes = match item.get_secret() {
            Ok(bytes) => Zeroizing::new(bytes),
            // Deleted between the search and the read: it is not there any more.
            Err(KeyringError::NoEntry) => continue,
            Err(error) => return Err(map_error(error).with_key(&key)),
        };
        all.insert(key.clone(), decode(bytes).map_err(|e| e.with_key(&key))?);
    }
    Ok(all)
}

fn delete_all(call: &Call) -> Result<(), SecureStorageError> {
    let store = call.store()?;
    for (key, item) in items(&*store, &call.account)? {
        match item.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => {}
            Err(error) => return Err(map_error(error).with_key(&key)),
        }
    }
    Ok(())
}

/// Every key of the store named `account`, with its item. The search matches the service
/// attribute; each item's names are checked again, so no other store's item is ever touched.
fn items(
    store: &CredentialStore,
    account: &str,
) -> Result<Vec<(String, Entry)>, SecureStorageError> {
    let spec = HashMap::from([("service", account)]);
    let found = store.search(&spec).map_err(map_error)?;
    let mut mine = Vec::new();
    for item in found {
        match item.get_specifiers() {
            Some((service, key)) if service == account => mine.push((key, item)),
            Some(_) => {}
            // An item of this store without its key name is not one this crate wrote.
            None => return Err(SecureStorageError::new(ErrorKind::Corrupt)),
        }
    }
    Ok(mine)
}

/// The value as text. A value that is not UTF-8 was not written by this crate: `Corrupt`.
fn decode(bytes: Zeroizing<Vec<u8>>) -> Result<Zeroizing<String>, SecureStorageError> {
    let text =
        std::str::from_utf8(&bytes).map_err(|_| SecureStorageError::new(ErrorKind::Corrupt))?;
    Ok(Zeroizing::new(text.to_owned()))
}

fn account_of(options: &PlatformOptions) -> Result<String, SecureStorageError> {
    let account = options.linux.account_name.clone();
    if account.is_empty() {
        return Err(SecureStorageError::new(ErrorKind::NotAvailable));
    }
    Ok(account)
}

/// Runs blocking Secret Service work off the engine thread.
async fn run<R: Send + 'static>(
    work: impl FnOnce() -> R + Send + 'static,
) -> Result<R, SecureStorageError> {
    nsh_core::prelude::blocking(work)
        .await
        .map_err(|_| SecureStorageError::new(ErrorKind::Backend))
}

impl Backend for LinuxBackend {
    fn store_id(&self, options: &PlatformOptions) -> String {
        match account_of(options) {
            Ok(account) => format!("linux|{account}"),
            Err(_) => "linux|unavailable".to_owned(),
        }
    }

    fn write<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
        value: &'a str,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        let call = account_of(options).map(|account| Call {
            source: Arc::clone(&self.source),
            account,
        });
        let key = key.to_owned();
        let value = Zeroizing::new(value.to_owned());
        Box::pin(async move {
            let call = call?;
            run(move || write(&call, &key, value.as_bytes())).await?
        })
    }

    fn read<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<Option<Zeroizing<String>>, SecureStorageError>> {
        let call = account_of(options).map(|account| Call {
            source: Arc::clone(&self.source),
            account,
        });
        let key = key.to_owned();
        Box::pin(async move {
            let call = call?;
            run(move || read(&call, &key)).await?
        })
    }

    fn contains_key<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<bool, SecureStorageError>> {
        let call = account_of(options).map(|account| Call {
            source: Arc::clone(&self.source),
            account,
        });
        let key = key.to_owned();
        Box::pin(async move {
            let call = call?;
            run(move || read(&call, &key).map(|value| value.is_some())).await?
        })
    }

    fn delete<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        let call = account_of(options).map(|account| Call {
            source: Arc::clone(&self.source),
            account,
        });
        let key = key.to_owned();
        Box::pin(async move {
            let call = call?;
            run(move || delete(&call, &key)).await?
        })
    }

    fn read_all<'a>(
        &'a self,
        options: &'a PlatformOptions,
    ) -> BackendFuture<'a, Result<BTreeMap<String, Zeroizing<String>>, SecureStorageError>> {
        let call = account_of(options).map(|account| Call {
            source: Arc::clone(&self.source),
            account,
        });
        Box::pin(async move {
            let call = call?;
            run(move || read_all(&call)).await?
        })
    }

    fn delete_all<'a>(
        &'a self,
        options: &'a PlatformOptions,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        let call = account_of(options).map(|account| Call {
            source: Arc::clone(&self.source),
            account,
        });
        Box::pin(async move {
            let call = call?;
            run(move || delete_all(&call)).await?
        })
    }
}
