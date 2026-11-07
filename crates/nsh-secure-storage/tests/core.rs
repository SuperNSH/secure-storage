//! The store's operations, listeners and ordering, on the memory backend. No UI is involved.

use std::{
    collections::BTreeMap,
    future::Future,
    pin::{Pin, pin},
    sync::{Arc, Mutex},
    task::{Context, Poll, Wake, Waker},
    thread,
};

use nsh_secure_storage::{
    AndroidOptions, Backend, BackendFuture, ErrorKind, MemoryBackend, ReadOptions, SecureStorage,
    SecureStorageError, WriteOptions,
};
use zeroize::Zeroizing;

/// A store operation's pending result, boxed so several can wait together.
type Pending = Pin<Box<dyn Future<Output = Result<(), SecureStorageError>> + 'static>>;

/// Runs one future to its end on this thread.
fn block_on<T>(future: impl Future<Output = T>) -> T {
    struct Unpark(thread::Thread);
    impl Wake for Unpark {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(Unpark(thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        if let Poll::Ready(value) = future.as_mut().poll(&mut cx) {
            return value;
        }
        thread::park();
    }
}

/// Polls every future, in turn, until all are done. Returns their results in order.
fn join_all<'a, T>(mut futures: Vec<Pin<Box<dyn Future<Output = T> + 'a>>>) -> Vec<T> {
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    let mut results: Vec<Option<T>> = futures.iter().map(|_| None).collect();
    while results.iter().any(Option::is_none) {
        for (slot, future) in results.iter_mut().zip(futures.iter_mut()) {
            if slot.is_none()
                && let Poll::Ready(value) = future.as_mut().poll(&mut cx)
            {
                *slot = Some(value);
            }
        }
    }
    results.into_iter().flatten().collect()
}

fn read_value(storage: SecureStorage, key: &str) -> Option<String> {
    block_on(storage.read(key, &ReadOptions::default()))
        .unwrap()
        .map(|value| value.to_string())
}

fn write(storage: SecureStorage, key: &str, value: Option<&str>) {
    block_on(storage.write(key, value, &WriteOptions::default())).unwrap();
}

#[test]
fn write_read_delete_round_trip() {
    let storage = SecureStorage::memory();
    assert_eq!(read_value(storage, "token"), None);
    write(storage, "token", Some("abc"));
    assert_eq!(read_value(storage, "token").as_deref(), Some("abc"));
    write(storage, "token", Some("def"));
    assert_eq!(read_value(storage, "token").as_deref(), Some("def"));
    block_on(storage.delete("token", &Default::default())).unwrap();
    assert_eq!(read_value(storage, "token"), None);
}

#[test]
fn read_all_and_delete_all() {
    let storage = SecureStorage::memory();
    write(storage, "b", Some("2"));
    write(storage, "a", Some("1"));
    let all = block_on(storage.read_all(&Default::default())).unwrap();
    let expected: BTreeMap<String, String> = [("a", "1"), ("b", "2")]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    let got: BTreeMap<String, String> = all.into_iter().map(|(k, v)| (k, v.to_string())).collect();
    assert_eq!(got, expected);

    block_on(storage.delete_all(&Default::default())).unwrap();
    assert!(
        block_on(storage.read_all(&Default::default()))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn contains_key() {
    let storage = SecureStorage::memory();
    assert!(!block_on(storage.contains_key("k", &Default::default())).unwrap());
    write(storage, "k", Some("v"));
    assert!(block_on(storage.contains_key("k", &Default::default())).unwrap());
}

#[test]
fn writing_none_deletes() {
    let storage = SecureStorage::memory();
    write(storage, "k", Some("v"));
    write(storage, "k", None);
    assert_eq!(read_value(storage, "k"), None);
    assert!(!block_on(storage.contains_key("k", &Default::default())).unwrap());
}

#[test]
fn empty_value_round_trips_and_is_not_absent() {
    let storage = SecureStorage::memory();
    write(storage, "k", Some(""));
    assert_eq!(read_value(storage, "k").as_deref(), Some(""));
    assert!(block_on(storage.contains_key("k", &Default::default())).unwrap());
}

#[test]
fn large_value_round_trips_on_every_platform() {
    let storage = SecureStorage::memory();
    let long = "x".repeat(1 << 20);
    write(storage, "big", Some(&long));
    assert_eq!(read_value(storage, "big").as_deref(), Some(long.as_str()));
}

#[test]
fn reserved_key_characters_round_trip() {
    let storage = SecureStorage::memory();
    let keys = [
        "with space",
        "slash/and\\backslash",
        "emoji-🔐",
        "quote\"and'apostrophe",
        "colon:and;semicolon",
        "../../traversal",
        "..",
        "été-日本",
    ];
    let long_key = "k".repeat(300);
    write(storage, &long_key, Some("long"));
    assert_eq!(read_value(storage, &long_key).as_deref(), Some("long"));
    for key in keys {
        write(storage, key, Some(key));
    }
    for key in keys {
        assert_eq!(read_value(storage, key).as_deref(), Some(key));
    }
}

#[test]
fn empty_and_nul_keys_are_refused() {
    let storage = SecureStorage::memory();
    let error = block_on(storage.write("", Some("v"), &WriteOptions::default())).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidKey);
    assert_eq!(error.code(), "ERR_SECURE_STORAGE_INVALID_KEY");
    let error = block_on(storage.read("a\0b", &ReadOptions::default())).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidKey);
}

#[test]
fn delete_all_touches_only_own_prefix() {
    let mine = SecureStorage::memory();
    let other = SecureStorage::memory();
    write(mine, "k", Some("mine"));
    write(other, "k", Some("other"));
    block_on(mine.delete_all(&Default::default())).unwrap();
    assert_eq!(read_value(mine, "k"), None);
    assert_eq!(read_value(other, "k").as_deref(), Some("other"));
}

#[test]
fn listener_runs_on_write_and_delete() {
    let storage = SecureStorage::memory();
    let seen: Arc<Mutex<Vec<Option<String>>>> = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    let _subscription = storage.register_listener("k", move |value| {
        log.lock().unwrap().push(value.map(str::to_owned));
    });
    write(storage, "k", Some("one"));
    write(storage, "other", Some("ignored"));
    write(storage, "k", None);
    assert_eq!(
        *seen.lock().unwrap(),
        vec![Some("one".to_owned()), None],
        "a listener hears its own key only, with the new value or None"
    );
}

#[test]
fn stop_ends_a_listener() {
    let storage = SecureStorage::memory();
    let calls = Arc::new(Mutex::new(0_u32));
    let count = calls.clone();
    let subscription = storage.register_listener("k", move |_| *count.lock().unwrap() += 1);
    write(storage, "k", Some("one"));
    subscription.stop();
    write(storage, "k", Some("two"));
    assert_eq!(*calls.lock().unwrap(), 1);
}

#[test]
fn listeners_are_per_surface() {
    let first = SecureStorage::memory();
    let second = SecureStorage::memory();
    let calls = Arc::new(Mutex::new(0_u32));
    let count = calls.clone();
    let _subscription = first.register_listener("k", move |_| *count.lock().unwrap() += 1);
    write(second, "k", Some("elsewhere"));
    assert_eq!(
        *calls.lock().unwrap(),
        0,
        "another surface's write must not reach it"
    );
    write(first, "k", Some("here"));
    assert_eq!(*calls.lock().unwrap(), 1);

    first.unregister_all_listeners_for_key("k");
    write(first, "k", Some("again"));
    assert_eq!(*calls.lock().unwrap(), 1);
}

/// A backend that logs each step and yields once inside each write, so interleaving shows.
struct ProbeBackend {
    inner: MemoryBackend,
    log: Arc<Mutex<Vec<String>>>,
}

/// Yields once, then is ready.
struct YieldOnce(bool);

impl Future for YieldOnce {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            return Poll::Ready(());
        }
        self.0 = true;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

impl Backend for ProbeBackend {
    fn store_id(&self, options: &nsh_secure_storage::PlatformOptions) -> String {
        self.inner.store_id(options)
    }

    fn write<'a>(
        &'a self,
        options: &'a nsh_secure_storage::PlatformOptions,
        key: &'a str,
        value: &'a str,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        Box::pin(async move {
            self.log
                .lock()
                .unwrap()
                .push(format!("start {key}={value}"));
            YieldOnce(false).await;
            let done = self.inner.write(options, key, value).await;
            self.log.lock().unwrap().push(format!("end {key}={value}"));
            done
        })
    }

    fn read<'a>(
        &'a self,
        options: &'a nsh_secure_storage::PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<Option<Zeroizing<String>>, SecureStorageError>> {
        self.inner.read(options, key)
    }

    fn contains_key<'a>(
        &'a self,
        options: &'a nsh_secure_storage::PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<bool, SecureStorageError>> {
        self.inner.contains_key(options, key)
    }

    fn delete<'a>(
        &'a self,
        options: &'a nsh_secure_storage::PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        self.inner.delete(options, key)
    }

    fn read_all<'a>(
        &'a self,
        options: &'a nsh_secure_storage::PlatformOptions,
    ) -> BackendFuture<'a, Result<BTreeMap<String, Zeroizing<String>>, SecureStorageError>> {
        self.inner.read_all(options)
    }

    fn delete_all<'a>(
        &'a self,
        options: &'a nsh_secure_storage::PlatformOptions,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        self.inner.delete_all(options)
    }
}

fn probe_backend() -> (Arc<ProbeBackend>, Arc<Mutex<Vec<String>>>) {
    let log = Arc::new(Mutex::new(Vec::new()));
    let backend = Arc::new(ProbeBackend {
        inner: MemoryBackend::new(),
        log: log.clone(),
    });
    (backend, log)
}

#[test]
fn concurrent_first_writes_all_land() {
    let storage = SecureStorage::memory();
    let futures: Vec<Pending> = (0..32)
        .map(|i| {
            let key = format!("first-{i}");
            Box::pin(async move {
                let value = format!("v{i}");
                storage
                    .write(&key, Some(&value), &WriteOptions::default())
                    .await
            }) as Pin<Box<dyn Future<Output = Result<(), SecureStorageError>>>>
        })
        .collect();
    for result in join_all(futures) {
        result.unwrap();
    }
    let all = block_on(storage.read_all(&Default::default())).unwrap();
    assert_eq!(all.len(), 32, "every first write must land on its own key");
    assert_eq!(read_value(storage, "first-31").as_deref(), Some("v31"));
}

#[test]
fn two_handles_same_store_serialize_init() {
    // Two handles on one backend: the writes must not interleave, and a read after both sees the later one.
    let (backend, log) = probe_backend();
    let first = SecureStorage::with_backend(backend.clone(), Default::default());
    let second = SecureStorage::with_backend(backend, Default::default());
    let futures: Vec<Pending> = vec![
        Box::pin(async move { first.write("k", Some("1"), &WriteOptions::default()).await }),
        Box::pin(async move { second.write("k", Some("2"), &WriteOptions::default()).await }),
    ];
    for result in join_all(futures) {
        result.unwrap();
    }
    assert_eq!(
        *log.lock().unwrap(),
        vec!["start k=1", "end k=1", "start k=2", "end k=2"],
        "one store's writes must run one at a time, in order"
    );
    assert_eq!(read_value(first, "k").as_deref(), Some("2"));
}

#[test]
fn error_text_never_contains_the_value() {
    const SECRET: &str = "SECRET-VALUE-7f3a";
    let storage = SecureStorage::memory();
    write(storage, "k", Some(SECRET));

    let debug_handle = format!("{storage:?}");
    assert!(!debug_handle.contains(SECRET));

    let error = block_on(storage.read("", &ReadOptions::default())).unwrap_err();
    assert!(!format!("{error}").contains(SECRET));
    assert!(!format!("{error:?}").contains(SECRET));

    let failing = SecureStorageError::new(ErrorKind::Backend)
        .with_key("k")
        .with_status(-1);
    let rendered = format!("{failing} / {failing:?}");
    assert!(rendered.contains("ERR_SECURE_STORAGE_BACKEND"));
    assert!(rendered.contains("status -1"));
    assert!(!rendered.contains(SECRET));
}

/// A memory store whose calls can be made to fail, as a transient platform error would.
struct FlakyBackend {
    inner: MemoryBackend,
    failing: std::sync::atomic::AtomicBool,
}

impl FlakyBackend {
    fn refuse(&self) -> Result<(), SecureStorageError> {
        if self.failing.load(std::sync::atomic::Ordering::SeqCst) {
            Err(SecureStorageError::new(ErrorKind::Backend).with_status(-25291))
        } else {
            Ok(())
        }
    }
}

impl Backend for FlakyBackend {
    fn store_id(&self, options: &nsh_secure_storage::PlatformOptions) -> String {
        self.inner.store_id(options)
    }

    fn write<'a>(
        &'a self,
        options: &'a nsh_secure_storage::PlatformOptions,
        key: &'a str,
        value: &'a str,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        Box::pin(async move {
            self.refuse()?;
            self.inner.write(options, key, value).await
        })
    }

    fn read<'a>(
        &'a self,
        options: &'a nsh_secure_storage::PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<Option<Zeroizing<String>>, SecureStorageError>> {
        Box::pin(async move {
            self.refuse()?;
            self.inner.read(options, key).await
        })
    }

    fn contains_key<'a>(
        &'a self,
        options: &'a nsh_secure_storage::PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<bool, SecureStorageError>> {
        Box::pin(async move {
            self.refuse()?;
            self.inner.contains_key(options, key).await
        })
    }

    fn delete<'a>(
        &'a self,
        options: &'a nsh_secure_storage::PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        Box::pin(async move {
            self.refuse()?;
            self.inner.delete(options, key).await
        })
    }

    fn read_all<'a>(
        &'a self,
        options: &'a nsh_secure_storage::PlatformOptions,
    ) -> BackendFuture<'a, Result<BTreeMap<String, Zeroizing<String>>, SecureStorageError>> {
        Box::pin(async move {
            self.refuse()?;
            self.inner.read_all(options).await
        })
    }

    fn delete_all<'a>(
        &'a self,
        options: &'a nsh_secure_storage::PlatformOptions,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        Box::pin(async move {
            self.refuse()?;
            self.inner.delete_all(options).await
        })
    }
}

#[test]
fn transient_lookup_failure_keeps_key_and_data() {
    let backend = Arc::new(FlakyBackend {
        inner: MemoryBackend::new(),
        failing: std::sync::atomic::AtomicBool::new(false),
    });
    let storage = SecureStorage::with_backend(backend.clone(), Default::default());
    write(storage, "k", Some("keep-me"));

    backend
        .failing
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let read = block_on(storage.read("k", &ReadOptions::default())).unwrap_err();
    assert_eq!(
        read.kind(),
        ErrorKind::Backend,
        "a transient failure is an error"
    );
    assert!(block_on(storage.contains_key("k", &Default::default())).is_err());
    assert!(block_on(storage.delete("k", &Default::default())).is_err());
    assert!(block_on(storage.write("k", None, &WriteOptions::default())).is_err());

    backend
        .failing
        .store(false, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(read_value(storage, "k").as_deref(), Some("keep-me"));
}

#[test]
fn contains_key_matches_read_on_every_platform() {
    let storage = SecureStorage::memory();
    let check = |key: &str| {
        let present = block_on(storage.contains_key(key, &Default::default())).unwrap();
        let value = read_value(storage, key);
        assert_eq!(
            present,
            value.is_some(),
            "containsKey and read disagree for {key:?}"
        );
    };
    check("absent");
    write(storage, "k", Some("v"));
    check("k");
    write(storage, "k", Some(""));
    check("k");
    write(storage, "k", None);
    check("k");
}

#[test]
fn read_and_delete_absent_key_are_not_errors() {
    let storage = SecureStorage::memory();
    assert_eq!(read_value(storage, "never-written"), None);
    block_on(storage.delete("never-written", &Default::default())).unwrap();
    block_on(storage.delete("never-written", &Default::default())).unwrap();
}

#[test]
fn read_during_write_never_returns_stale_or_null() {
    let (backend, _log) = probe_backend();
    let storage = SecureStorage::with_backend(backend, Default::default());
    write(storage, "k", Some("old"));
    let futures: Vec<Pending> = vec![
        Box::pin(async move {
            storage
                .write("k", Some("new"), &WriteOptions::default())
                .await
        }),
        Box::pin(async move {
            let seen = storage.read("k", &ReadOptions::default()).await?;
            let seen = seen.map(|value| value.to_string());
            assert!(
                matches!(seen.as_deref(), Some("old") | Some("new")),
                "read saw {seen:?}"
            );
            Ok(())
        }),
    ];
    for result in join_all(futures) {
        result.unwrap();
    }
    assert_eq!(read_value(storage, "k").as_deref(), Some("new"));
}

#[test]
fn backend_load_failure_is_coded_error() {
    /// Every call fails the way a backend that did not load does.
    struct Unloaded;
    impl Backend for Unloaded {
        fn store_id(&self, _options: &nsh_secure_storage::PlatformOptions) -> String {
            "unloaded".to_owned()
        }
        fn write<'a>(
            &'a self,
            _options: &'a nsh_secure_storage::PlatformOptions,
            _key: &'a str,
            _value: &'a str,
        ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
            Box::pin(async { Err(SecureStorageError::new(ErrorKind::Backend)) })
        }
        fn read<'a>(
            &'a self,
            _options: &'a nsh_secure_storage::PlatformOptions,
            _key: &'a str,
        ) -> BackendFuture<'a, Result<Option<Zeroizing<String>>, SecureStorageError>> {
            Box::pin(async { Err(SecureStorageError::new(ErrorKind::Backend)) })
        }
        fn contains_key<'a>(
            &'a self,
            _options: &'a nsh_secure_storage::PlatformOptions,
            _key: &'a str,
        ) -> BackendFuture<'a, Result<bool, SecureStorageError>> {
            Box::pin(async { Err(SecureStorageError::new(ErrorKind::Backend)) })
        }
        fn delete<'a>(
            &'a self,
            _options: &'a nsh_secure_storage::PlatformOptions,
            _key: &'a str,
        ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
            Box::pin(async { Err(SecureStorageError::new(ErrorKind::Backend)) })
        }
        fn read_all<'a>(
            &'a self,
            _options: &'a nsh_secure_storage::PlatformOptions,
        ) -> BackendFuture<'a, Result<BTreeMap<String, Zeroizing<String>>, SecureStorageError>>
        {
            Box::pin(async { Err(SecureStorageError::new(ErrorKind::Backend)) })
        }
        fn delete_all<'a>(
            &'a self,
            _options: &'a nsh_secure_storage::PlatformOptions,
        ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
            Box::pin(async { Err(SecureStorageError::new(ErrorKind::Backend)) })
        }
    }
    let storage = SecureStorage::with_backend(Arc::new(Unloaded), Default::default());
    let error = block_on(storage.read("k", &ReadOptions::default())).unwrap_err();
    assert_eq!(error.code(), "ERR_SECURE_STORAGE_BACKEND");
    assert_eq!(error.key(), Some("k"), "the key is named");
}

#[test]
fn reset_on_error_defaults_to_false() {
    assert!(!AndroidOptions::default().reset_on_error);
}
