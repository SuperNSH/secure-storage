//! Secure key-value storage for Native Shell apps, on each platform's own secret store.
//!
//! [`SecureStorage`] is a `Copy` handle to one store. Values are read and written asynchronously,
//! so the engine thread never waits on the platform. [`use_secure_value`] reads one key as a
//! reactive value that follows the writes made through the same store.

#[cfg(any(target_os = "android", test))]
mod android;
#[cfg(any(target_os = "macos", target_os = "ios"))]
mod apple;
mod backend;
mod error;
#[cfg(not(target_arch = "wasm32"))]
mod keyring_store;
#[cfg(not(target_arch = "wasm32"))]
mod linux;
mod memory;
mod native;
mod options;
mod reactive;
mod store;
#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "linux",
    target_os = "android",
    target_arch = "wasm32"
)))]
mod unsupported;
#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(not(target_arch = "wasm32"))]
mod windows;

pub use backend::{Backend, BackendFuture};
pub use error::{ErrorKind, SecureStorageError};
pub use memory::MemoryBackend;
pub use options::{
    AccessControlFlag, AndroidBiometricType, AndroidOptions, AppleOptions, AuthenticationUi,
    CallOptions, ContainsKeyOptions, DeleteAllOptions, DeleteOptions, IosOptions,
    KeychainAccessibility, LinuxOptions, MacOsOptions, PlatformOptions, ReadAllOptions,
    ReadOptions, SecureStorageOptions, WebOptions, WindowsOptions, WriteOptions,
};
pub use reactive::{SecureValue, SecureValueState, use_secure_value};
pub use store::{SecureStorage, Subscription};

#[cfg(not(target_arch = "wasm32"))]
pub use linux::LinuxBackend;
pub use native::{NativeSecureStorage, StoreOptions, StoredEntry};
#[cfg(target_arch = "wasm32")]
pub use web::WebBackend;
#[cfg(not(target_arch = "wasm32"))]
pub use windows::WindowsBackend;

use nsh_core::prelude::*;

/// The crate's UI set: what TSX imports from `nsh-secure-storage/native`.
#[native_module("nsh-secure-storage/native")]
fn native(registry: &mut Registry) {
    registry.class::<NativeSecureStorage>();
}
