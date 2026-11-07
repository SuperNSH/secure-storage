#![cfg(target_os = "linux")]

//! A locked Secret Service reports a code, never a missing key. Runs only in the Docker image,
//! in its own process, because it locks the keyring for every test that follows it.

use std::{future::Future, process::Command, sync::Arc};

use nsh_secure_storage::{
    LinuxBackend, PlatformOptions, ReadOptions, SecureStorage, SecureStorageOptions, WriteOptions,
};

fn block_on<T>(future: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime for the blocking pool")
        .block_on(future)
}

fn storage(account: &str) -> SecureStorage {
    let mut platform = PlatformOptions::default();
    platform.linux.account_name = account.to_owned();
    let mut options = SecureStorageOptions::default();
    options.platform = platform;
    SecureStorage::with_backend(Arc::new(LinuxBackend::system()), options)
}

#[test]
fn linux_secret_service_locked_keyring_is_coded_error() {
    if std::env::var_os("NSH_SECURE_STORAGE_LINUX_TESTS").as_deref() != Some("1".as_ref()) {
        eprintln!("skipped: set NSH_SECURE_STORAGE_LINUX_TESTS=1 inside the Docker image");
        return;
    }
    let store = storage(&format!("nsh-test-locked-{}", std::process::id()));
    block_on(store.write("k", Some("v"), &WriteOptions::default())).unwrap();

    // Lock the default collection over D-Bus. No prompter runs here, so an unlock cannot succeed.
    let status = Command::new("dbus-send")
        .args([
            "--session",
            "--print-reply",
            "--dest=org.freedesktop.secrets",
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service.Lock",
            "array:objpath:/org/freedesktop/secrets/collection/login",
        ])
        .status()
        .expect("dbus-send runs");
    assert!(status.success(), "the collection is locked");

    let error = block_on(store.read("k", &ReadOptions::default()))
        .expect_err("a locked keyring is an error, not a missing key");
    assert_eq!(error.code(), "ERR_SECURE_STORAGE_LOCKED", "{error}");
}
