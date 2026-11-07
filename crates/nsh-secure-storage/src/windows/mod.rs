//! The Windows backend. Credential Manager holds one AES-256 key per store, since its values are
//! size-limited. The values sit in one AES-256-GCM file per store under `%LOCALAPPDATA%`.

mod values;

use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::Arc,
};

use keyring_core::{CredentialStore, Entry, Error as KeyringError};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::{
    backend::{Backend, BackendFuture},
    error::{ErrorKind, SecureStorageError},
    keyring_store::{encode_name, map_error},
    options::PlatformOptions,
};
use values::{ValuesFile, open, remove_file, seal};

/// The Credential Manager target of a key is this prefix, then the store, a `/`, and the app.
#[cfg(windows)]
const TARGET_PREFIX: &str = "nsh-secure-storage/";
const KEY_LEN: usize = 32;
/// The longest encoded name a Credential Manager target or a file name can carry in full.
const MAX_ENCODED_NAME: usize = 200;

/// The backend for Windows. Its key entries live in `store`, and its values under `root`.
pub struct WindowsBackend {
    store: Arc<CredentialStore>,
    /// `None` when `%LOCALAPPDATA%` is not set: every call then reports `NotAvailable`.
    root: Option<PathBuf>,
    /// The Credential Manager persistence of a new key. `Local` keeps it on this machine.
    persistence: Option<&'static str>,
}

impl WindowsBackend {
    /// A backend whose keys live in `store` and whose values live under `root`. Each app's
    /// values sit in their own folder under `root`. Apps and tests use this to supply a store.
    pub fn new(store: Arc<CredentialStore>, root: PathBuf) -> Self {
        Self {
            store,
            root: Some(root),
            persistence: None,
        }
    }

    /// The backend on this user's Credential Manager, with values under `%LOCALAPPDATA%`.
    #[cfg(windows)]
    pub fn system() -> Result<Self, SecureStorageError> {
        let config = HashMap::from([
            ("prefix", TARGET_PREFIX),
            ("divider", "/"),
            ("suffix", ""),
            ("service_no_divider", "true"),
        ]);
        let store: Arc<CredentialStore> =
            windows_native_keyring_store::Store::new_with_configuration(&config)
                .map_err(map_error)?;
        let root = std::env::var_os("LOCALAPPDATA")
            .map(|local| PathBuf::from(local).join("nsh-secure-storage"));
        Ok(Self {
            store,
            root,
            persistence: Some("Local"),
        })
    }

    /// Names one store: its app and its store name, and where its values go.
    fn context(&self, options: &PlatformOptions) -> Result<Context, SecureStorageError> {
        let app = options
            .windows
            .app_id
            .clone()
            .or_else(process_app_id)
            .filter(|id| !id.is_empty())
            .ok_or_else(not_available)?;
        let account = options.windows.account_name.clone();
        let service = encode_name(&app);
        let user = encode_name(&account);
        if account.is_empty() || service.len() > MAX_ENCODED_NAME || user.len() > MAX_ENCODED_NAME {
            return Err(not_available());
        }
        let root = self.root.clone().ok_or_else(not_available)?;
        let app_dir = root.join(hex(&Sha256::digest(app.as_bytes())));
        let file = app_dir.join(format!(
            "{}.values",
            hex(&Sha256::digest(account.as_bytes()))
        ));
        Ok(Context {
            store: Arc::clone(&self.store),
            persistence: self.persistence,
            service,
            user,
            app,
            account,
            file,
        })
    }
}

/// What one call addresses. Owned, so the blocking work can run with it.
#[derive(Clone)]
struct Context {
    store: Arc<CredentialStore>,
    persistence: Option<&'static str>,
    /// The Credential Manager service: the encoded app identity.
    service: String,
    /// The Credential Manager user: the encoded store name.
    user: String,
    app: String,
    account: String,
    /// The values file of this store.
    file: PathBuf,
}

impl Context {
    /// The Credential Manager entry of this store's key. Building it does not touch the store.
    fn entry(&self) -> Result<Entry, SecureStorageError> {
        let modifiers = self
            .persistence
            .map(|value| HashMap::from([("persistence", value)]));
        self.store
            .build(&self.service, &self.user, modifiers.as_ref())
            .map_err(map_error)
    }

    /// This store's AES key. With `create`, a missing key is made; without, it is `None`.
    ///
    /// A failed lookup is an error and never a new key: a new key cannot read the old values.
    fn key(&self, create: bool) -> Result<Option<Zeroizing<[u8; KEY_LEN]>>, SecureStorageError> {
        let entry = self.entry()?;
        match entry.get_secret() {
            Ok(bytes) => {
                let bytes = Zeroizing::new(bytes);
                let key = <[u8; KEY_LEN]>::try_from(bytes.as_slice())
                    .map_err(|_| SecureStorageError::new(ErrorKind::Corrupt))?;
                Ok(Some(Zeroizing::new(key)))
            }
            Err(KeyringError::NoEntry) if create => {
                let mut key = Zeroizing::new([0u8; KEY_LEN]);
                getrandom::fill(&mut key[..])
                    .map_err(|_| SecureStorageError::new(ErrorKind::Backend))?;
                entry.set_secret(&key[..]).map_err(map_error)?;
                Ok(Some(key))
            }
            Err(KeyringError::NoEntry) => Ok(None),
            Err(error) => Err(map_error(error)),
        }
    }

    /// The key of an existing record. Its absence means the values cannot be read.
    fn existing_key(&self) -> Result<Zeroizing<[u8; KEY_LEN]>, SecureStorageError> {
        self.key(false)?
            .ok_or_else(|| SecureStorageError::new(ErrorKind::KeyInvalidated))
    }

    /// The data bound to a record: the app, the store and the key name, so a record cannot be
    /// moved to another key, store or app and still open.
    fn associated_data(&self, key: &str) -> Vec<u8> {
        let mut aad = Vec::new();
        for part in [self.app.as_bytes(), self.account.as_bytes(), key.as_bytes()] {
            aad.extend_from_slice(&(part.len() as u64).to_le_bytes());
            aad.extend_from_slice(part);
        }
        aad
    }
}

fn write(context: &Context, key: &str, value: &str) -> Result<(), SecureStorageError> {
    let mut file = ValuesFile::load(&context.file)?;
    // A new key is made only for an empty file: records under a lost key cannot be opened.
    let secret = if file.is_empty() {
        context.key(true)?.ok_or_else(not_available)?
    } else {
        context.existing_key()?
    };
    let record = seal(&secret, &context.associated_data(key), value.as_bytes())?;
    file.insert(key, record);
    file.save(&context.file)
}

fn read(context: &Context, key: &str) -> Result<Option<Zeroizing<String>>, SecureStorageError> {
    let file = ValuesFile::load(&context.file)?;
    let Some(record) = file.get(key) else {
        return Ok(None);
    };
    let secret = context.existing_key()?;
    open(&secret, &context.associated_data(key), record).map(Some)
}

fn delete(context: &Context, key: &str) -> Result<(), SecureStorageError> {
    let mut file = ValuesFile::load(&context.file)?;
    if file.remove(key) {
        file.save(&context.file)?;
    }
    Ok(())
}

fn read_all(context: &Context) -> Result<BTreeMap<String, Zeroizing<String>>, SecureStorageError> {
    let file = ValuesFile::load(&context.file)?;
    if file.is_empty() {
        return Ok(BTreeMap::new());
    }
    let secret = context.existing_key()?;
    file.iter()
        .map(|(key, record)| {
            open(&secret, &context.associated_data(key), record).map(|value| (key.clone(), value))
        })
        .collect()
}

fn delete_all(context: &Context) -> Result<(), SecureStorageError> {
    remove_file(&context.file)
}

/// Runs blocking Credential Manager and file work off the engine thread.
async fn run<R: Send + 'static>(
    work: impl FnOnce() -> R + Send + 'static,
) -> Result<R, SecureStorageError> {
    nsh_core::prelude::blocking(work)
        .await
        .map_err(|_| SecureStorageError::new(ErrorKind::Backend))
}

impl Backend for WindowsBackend {
    fn store_id(&self, options: &PlatformOptions) -> String {
        match self.context(options) {
            Ok(context) => format!("windows|{}|{}", context.service, context.user),
            Err(_) => "windows|unavailable".to_owned(),
        }
    }

    fn write<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
        value: &'a str,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        let context = self.context(options);
        let key = key.to_owned();
        let value = Zeroizing::new(value.to_owned());
        Box::pin(async move {
            let context = context?;
            run(move || write(&context, &key, &value)).await?
        })
    }

    fn read<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<Option<Zeroizing<String>>, SecureStorageError>> {
        let context = self.context(options);
        let key = key.to_owned();
        Box::pin(async move {
            let context = context?;
            run(move || read(&context, &key)).await?
        })
    }

    fn contains_key<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<bool, SecureStorageError>> {
        let context = self.context(options);
        let key = key.to_owned();
        Box::pin(async move {
            let context = context?;
            run(move || read(&context, &key).map(|value| value.is_some())).await?
        })
    }

    fn delete<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        let context = self.context(options);
        let key = key.to_owned();
        Box::pin(async move {
            let context = context?;
            run(move || delete(&context, &key)).await?
        })
    }

    fn read_all<'a>(
        &'a self,
        options: &'a PlatformOptions,
    ) -> BackendFuture<'a, Result<BTreeMap<String, Zeroizing<String>>, SecureStorageError>> {
        let context = self.context(options);
        Box::pin(async move {
            let context = context?;
            run(move || read_all(&context)).await?
        })
    }

    fn delete_all<'a>(
        &'a self,
        options: &'a PlatformOptions,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        let context = self.context(options);
        Box::pin(async move {
            let context = context?;
            run(move || delete_all(&context)).await?
        })
    }
}

fn not_available() -> SecureStorageError {
    SecureStorageError::new(ErrorKind::NotAvailable)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The app identity NSH gives this process: its AppUserModelID, set from `nsh.json`'s `id`.
#[cfg(windows)]
fn process_app_id() -> Option<String> {
    use std::ffi::c_void;
    use windows_sys::Win32::{
        System::Com::CoTaskMemFree, UI::Shell::GetCurrentProcessExplicitAppUserModelID,
    };

    let mut raw = std::ptr::null_mut();
    // SAFETY: on success the call sets `raw` to a string it allocated, which is freed below.
    let status = unsafe { GetCurrentProcessExplicitAppUserModelID(&mut raw) };
    if status != 0 || raw.is_null() {
        return None;
    }
    // SAFETY: `raw` is a NUL-terminated UTF-16 string that the call allocated.
    let id = unsafe {
        let length = (0..).take_while(|&i| *raw.add(i) != 0).count();
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(raw, length));
        CoTaskMemFree(raw as *const c_void);
        text
    };
    Some(id).filter(|id| !id.is_empty())
}

/// Only Windows has an AppUserModelID; elsewhere the app must set `windows.app_id`.
#[cfg(not(windows))]
fn process_app_id() -> Option<String> {
    None
}
