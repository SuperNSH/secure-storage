//! The backend of a platform that has none yet. Every call reports `NotAvailable`, so nothing
//! is stored in the clear by accident.

use std::collections::BTreeMap;

use zeroize::Zeroizing;

use crate::{
    backend::{Backend, BackendFuture},
    error::{ErrorKind, SecureStorageError},
    options::PlatformOptions,
};

/// A backend that refuses every call.
pub struct UnsupportedBackend;

fn refuse<'a, T: 'a>() -> BackendFuture<'a, Result<T, SecureStorageError>> {
    Box::pin(async { Err(SecureStorageError::new(ErrorKind::NotAvailable)) })
}

impl Backend for UnsupportedBackend {
    fn store_id(&self, _options: &PlatformOptions) -> String {
        "unsupported".to_owned()
    }

    fn write<'a>(
        &'a self,
        _options: &'a PlatformOptions,
        _key: &'a str,
        _value: &'a str,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        refuse()
    }

    fn read<'a>(
        &'a self,
        _options: &'a PlatformOptions,
        _key: &'a str,
    ) -> BackendFuture<'a, Result<Option<Zeroizing<String>>, SecureStorageError>> {
        refuse()
    }

    fn contains_key<'a>(
        &'a self,
        _options: &'a PlatformOptions,
        _key: &'a str,
    ) -> BackendFuture<'a, Result<bool, SecureStorageError>> {
        refuse()
    }

    fn delete<'a>(
        &'a self,
        _options: &'a PlatformOptions,
        _key: &'a str,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        refuse()
    }

    fn read_all<'a>(
        &'a self,
        _options: &'a PlatformOptions,
    ) -> BackendFuture<'a, Result<BTreeMap<String, Zeroizing<String>>, SecureStorageError>> {
        refuse()
    }

    fn delete_all<'a>(
        &'a self,
        _options: &'a PlatformOptions,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        refuse()
    }
}
