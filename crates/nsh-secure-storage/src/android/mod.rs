//! The Android backend: AES-GCM under a non-exportable AndroidKeyStore key, and one file per
//! store in the app's no-backup folder, so backup and device transfer leave the ciphertext out.
//!
//! The store logic is plain Rust and runs on any host. Only the JNI Keystore is Android-only.

mod file;
#[cfg(target_os = "android")]
mod jni;
mod keystore;
mod store;
#[cfg(test)]
mod tests;

use std::{collections::BTreeMap, future::Future, sync::Arc};

use zeroize::Zeroizing;

use crate::{
    backend::{Backend, BackendFuture},
    error::{ErrorKind, SecureStorageError},
    options::PlatformOptions,
};
use keystore::{Keystore, Policy};

/// The Android backend. It keeps no state of its own: each call names its store in its options.
pub(crate) struct AndroidBackend {
    keystore: Arc<dyn Keystore>,
}

impl AndroidBackend {
    /// The backend on this device's AndroidKeyStore.
    #[cfg(target_os = "android")]
    pub(crate) fn new() -> Self {
        Self::with_keystore(Arc::new(jni::Jni))
    }

    /// The backend on `keystore`. Tests use a fake here.
    pub(crate) fn with_keystore(keystore: Arc<dyn Keystore>) -> Self {
        Self { keystore }
    }
}

/// Runs blocking Keystore and file work off the engine thread. A panic on that thread is a
/// backend failure, and the store stays usable.
async fn run<R: Send + 'static>(
    work: impl FnOnce() -> R + Send + 'static,
) -> Result<R, SecureStorageError> {
    nsh_core::prelude::blocking(work)
        .await
        .map_err(|_| SecureStorageError::new(ErrorKind::Backend))
}

/// Boxes a call's work, so each method of [`Backend`] reads the same way.
fn call<R: Send + 'static>(
    backend: &AndroidBackend,
    options: &PlatformOptions,
    work: impl FnOnce(Arc<dyn Keystore>, Policy) -> R + Send + 'static,
) -> impl Future<Output = Result<R, SecureStorageError>> {
    let keystore = backend.keystore.clone();
    let policy = Policy::from_options(options);
    async move { run(move || work(keystore, policy)).await }
}

impl Backend for AndroidBackend {
    fn store_id(&self, options: &PlatformOptions) -> String {
        Policy::from_options(options).store_id()
    }

    fn write<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
        value: &'a str,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        let key = key.to_owned();
        let value = Zeroizing::new(value.to_owned());
        let work = call(self, options, move |keystore, policy| {
            store::write(&*keystore, &policy, &key, &value)
        });
        Box::pin(async move { work.await? })
    }

    fn read<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<Option<Zeroizing<String>>, SecureStorageError>> {
        let key = key.to_owned();
        let work = call(self, options, move |keystore, policy| {
            store::read(&*keystore, &policy, &key)
        });
        Box::pin(async move { work.await? })
    }

    fn contains_key<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<bool, SecureStorageError>> {
        let key = key.to_owned();
        let work = call(self, options, move |keystore, policy| {
            store::contains_key(&*keystore, &policy, &key)
        });
        Box::pin(async move { work.await? })
    }

    fn delete<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        let key = key.to_owned();
        let work = call(self, options, move |keystore, policy| {
            store::delete(&*keystore, &policy, &key)
        });
        Box::pin(async move { work.await? })
    }

    fn read_all<'a>(
        &'a self,
        options: &'a PlatformOptions,
    ) -> BackendFuture<'a, Result<BTreeMap<String, Zeroizing<String>>, SecureStorageError>> {
        let work = call(self, options, move |keystore, policy| {
            store::read_all(&*keystore, &policy)
        });
        Box::pin(async move { work.await? })
    }

    fn delete_all<'a>(
        &'a self,
        options: &'a PlatformOptions,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        let work = call(self, options, move |keystore, policy| {
            store::reset(&*keystore, &policy)
        });
        Box::pin(async move { work.await? })
    }

    /// Removes the store's file and key. The core calls it once, after a `KeyInvalidated` with
    /// `reset_on_error`, and retries the call once.
    fn reset_store<'a>(
        &'a self,
        options: &'a PlatformOptions,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        let work = call(self, options, move |keystore, policy| {
            store::reset(&*keystore, &policy)
        });
        Box::pin(async move { work.await? })
    }
}
