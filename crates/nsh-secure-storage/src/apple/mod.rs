//! The Apple backend: the Keychain, on iOS and macOS.
//!
//! Each call runs on a blocking thread through `nsh_core::blocking`. The items are generic
//! passwords filed under the account name as their service.

mod ops;
#[cfg(target_os = "macos")]
mod prompt;
#[cfg(target_os = "ios")]
mod protected;
mod status;
mod sys;

use std::collections::BTreeMap;
#[cfg(target_os = "ios")]
use std::rc::Rc;

use zeroize::Zeroizing;

use crate::{
    backend::{Backend, BackendFuture},
    error::{ErrorKind, SecureStorageError},
    options::PlatformOptions,
};
use ops::Spec;

/// The Keychain as a backend. It keeps no state: each call names its store in its options.
pub(crate) struct AppleBackend;

/// Runs blocking Keychain work off the engine thread. A panic on that thread is a backend failure.
async fn run<R: Send + 'static>(
    work: impl FnOnce() -> R + Send + 'static,
) -> Result<R, SecureStorageError> {
    nsh_core::prelude::blocking(work)
        .await
        .map_err(|_| SecureStorageError::new(ErrorKind::Backend))
}

impl Backend for AppleBackend {
    fn store_id(&self, options: &PlatformOptions) -> String {
        match Spec::from_options(options) {
            Ok(spec) => spec.identity(),
            Err(_) => "apple|unavailable".to_owned(),
        }
    }

    fn write<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
        value: &'a str,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        let spec = Spec::from_options(options);
        let key = key.to_owned();
        let value = Zeroizing::new(value.to_owned());
        Box::pin(async move {
            let spec = spec?;
            run(move || ops::write(&spec, &key, &value)).await?
        })
    }

    fn read<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<Option<Zeroizing<String>>, SecureStorageError>> {
        let spec = Spec::from_options(options);
        let key = key.to_owned();
        Box::pin(async move {
            let spec = spec?;
            run(move || ops::read(&spec, &key)).await?
        })
    }

    fn contains_key<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<bool, SecureStorageError>> {
        let spec = Spec::from_options(options);
        let key = key.to_owned();
        Box::pin(async move {
            let spec = spec?;
            run(move || ops::contains_key(&spec, &key)).await?
        })
    }

    fn delete<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        let spec = Spec::from_options(options);
        let key = key.to_owned();
        Box::pin(async move {
            let spec = spec?;
            run(move || ops::delete(&spec, &key)).await?
        })
    }

    fn read_all<'a>(
        &'a self,
        options: &'a PlatformOptions,
    ) -> BackendFuture<'a, Result<BTreeMap<String, Zeroizing<String>>, SecureStorageError>> {
        let spec = Spec::from_options(options);
        Box::pin(async move {
            let spec = spec?;
            run(move || ops::read_all(&spec)).await?
        })
    }

    fn delete_all<'a>(
        &'a self,
        options: &'a PlatformOptions,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        let spec = Spec::from_options(options);
        Box::pin(async move {
            let spec = spec?;
            run(move || ops::delete_all(&spec)).await?
        })
    }

    #[cfg(target_os = "ios")]
    fn is_protected_data_available(&self) -> bool {
        protected::is_available()
    }

    #[cfg(target_os = "ios")]
    fn on_protected_data_change(&self, listener: Rc<dyn Fn(bool)>) -> Box<dyn FnOnce()> {
        protected::subscribe(listener)
    }
}

#[cfg(test)]
mod tests {
    /// Every Keychain call goes through `run`, so its work must leave the engine's thread.
    #[test]
    fn backend_calls_run_off_engine_thread() {
        let engine_thread = std::thread::current().id();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime for the blocking pool");
        let ran_on = runtime
            .block_on(super::run(|| std::thread::current().id()))
            .expect("the work finishes");
        assert_ne!(
            ran_on, engine_thread,
            "Keychain work ran on the engine thread"
        );
    }
}
