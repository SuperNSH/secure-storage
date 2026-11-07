#![cfg(target_os = "linux")]

//! With no Secret Service on the bus, every call reports a code. Runs only in the Docker image,
//! in its own process: it points the process's session bus at a socket that does not exist.

use std::{future::Future, sync::Arc};

use nsh_secure_storage::{
    LinuxBackend, PlatformOptions, ReadOptions, SecureStorage, SecureStorageOptions,
};

fn block_on<T>(future: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime for the blocking pool")
        .block_on(future)
}

#[test]
fn linux_no_secret_service_is_coded_error() {
    if std::env::var_os("NSH_SECURE_STORAGE_LINUX_TESTS").as_deref() != Some("1".as_ref()) {
        eprintln!("skipped: set NSH_SECURE_STORAGE_LINUX_TESTS=1 inside the Docker image");
        return;
    }
    // SAFETY: this test binary has one test, and it sets the variable before any thread reads it.
    unsafe {
        std::env::set_var(
            "DBUS_SESSION_BUS_ADDRESS",
            "unix:path=/nonexistent/nsh-secure-storage-test-bus",
        );
    }
    let mut options = SecureStorageOptions::default();
    options.platform = PlatformOptions::default();
    let store = SecureStorage::with_backend(Arc::new(LinuxBackend::system()), options);
    let error = block_on(store.read("k", &ReadOptions::default()))
        .expect_err("no Secret Service is an error");
    assert_eq!(error.code(), "ERR_SECURE_STORAGE_NOT_AVAILABLE", "{error}");
}
