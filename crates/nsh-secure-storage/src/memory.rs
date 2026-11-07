//! The memory backend: a map in the process. Nothing reaches the device.

use std::{
    collections::BTreeMap,
    sync::{Mutex, MutexGuard},
};

use zeroize::Zeroizing;

use crate::{
    backend::{Backend, BackendFuture},
    error::SecureStorageError,
    options::PlatformOptions,
};

/// A map of values in memory. It ignores the platform options: one map is one store.
#[derive(Default)]
pub struct MemoryBackend {
    entries: Mutex<BTreeMap<String, Zeroizing<String>>>,
}

impl MemoryBackend {
    /// An empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// A map that starts with `values`.
    pub fn with_initial_values(values: BTreeMap<String, String>) -> Self {
        let entries = values
            .into_iter()
            .map(|(key, value)| (key, Zeroizing::new(value)))
            .collect();
        Self {
            entries: Mutex::new(entries),
        }
    }

    fn entries(&self) -> MutexGuard<'_, BTreeMap<String, Zeroizing<String>>> {
        self.entries.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Backend for MemoryBackend {
    fn store_id(&self, _options: &PlatformOptions) -> String {
        format!("memory:{:p}", self)
    }

    fn write<'a>(
        &'a self,
        _options: &'a PlatformOptions,
        key: &'a str,
        value: &'a str,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        Box::pin(async move {
            self.entries().insert(key.to_owned(), Zeroizing::new(value.to_owned()));
            Ok(())
        })
    }

    fn read<'a>(
        &'a self,
        _options: &'a PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<Option<Zeroizing<String>>, SecureStorageError>> {
        Box::pin(async move { Ok(self.entries().get(key).cloned()) })
    }

    fn contains_key<'a>(
        &'a self,
        _options: &'a PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<bool, SecureStorageError>> {
        Box::pin(async move { Ok(self.entries().contains_key(key)) })
    }

    fn delete<'a>(
        &'a self,
        _options: &'a PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        Box::pin(async move {
            self.entries().remove(key);
            Ok(())
        })
    }

    fn read_all<'a>(
        &'a self,
        _options: &'a PlatformOptions,
    ) -> BackendFuture<'a, Result<BTreeMap<String, Zeroizing<String>>, SecureStorageError>> {
        Box::pin(async move { Ok(self.entries().clone()) })
    }

    fn delete_all<'a>(
        &'a self,
        _options: &'a PlatformOptions,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        Box::pin(async move {
            self.entries().clear();
            Ok(())
        })
    }
}
