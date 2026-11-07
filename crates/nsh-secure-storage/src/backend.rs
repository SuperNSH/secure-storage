//! The seam every platform implements. A backend only stores; the core checks keys and runs
//! the listeners.

use std::{collections::BTreeMap, future::Future, pin::Pin, rc::Rc};

use zeroize::Zeroizing;

use crate::{error::SecureStorageError, options::PlatformOptions};

/// A backend's pending operation. Not `Send`, so the web can implement it too.
pub type BackendFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// Where the bytes of a store live: a platform's secret store, or a memory map.
///
/// Native backends run their blocking calls through `nsh_core::blocking`, so the engine thread
/// does not wait on them.
pub trait Backend: Send + Sync {
    /// Names the store the options address. Two handles whose ids match share one lock, so their
    /// operations run in order.
    fn store_id(&self, options: &PlatformOptions) -> String;

    /// Stores `value` under `key`, replacing any value there.
    fn write<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
        value: &'a str,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>>;

    /// The value under `key`, or `None` when there is none.
    fn read<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<Option<Zeroizing<String>>, SecureStorageError>>;

    /// Whether a value is stored under `key`.
    fn contains_key<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<bool, SecureStorageError>>;

    /// Removes the value under `key`. Nothing happens when there is none.
    fn delete<'a>(
        &'a self,
        options: &'a PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>>;

    /// Every key and value of the store, in key order.
    fn read_all<'a>(
        &'a self,
        options: &'a PlatformOptions,
    ) -> BackendFuture<'a, Result<BTreeMap<String, Zeroizing<String>>, SecureStorageError>>;

    /// Removes every value of the store, and nothing outside it.
    fn delete_all<'a>(
        &'a self,
        options: &'a PlatformOptions,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>>;

    /// Whether the platform's protected data is readable now. Only iOS can say no.
    fn is_protected_data_available(&self) -> bool {
        true
    }

    /// Calls `listener` when protected data becomes available or unavailable, until the returned
    /// function runs. Only iOS reports changes.
    fn on_protected_data_change(&self, _listener: Rc<dyn Fn(bool)>) -> Box<dyn FnOnce()> {
        Box::new(|| {})
    }
}
