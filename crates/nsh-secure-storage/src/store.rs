//! The store handle: keys, options, per-store ordering and listeners. Platforms only store.

use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, HashMap},
    panic::AssertUnwindSafe,
    rc::Rc,
    sync::{Arc, Mutex, OnceLock, Weak},
    task::Poll,
};

use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};
use zeroize::Zeroizing;

use crate::{
    backend::{Backend, BackendFuture},
    error::{ErrorKind, SecureStorageError},
    options::{
        CallOptions, ContainsKeyOptions, DeleteAllOptions, DeleteOptions, PlatformOptions,
        ReadAllOptions, ReadOptions, SecureStorageOptions, WriteOptions,
    },
};

/// A store of secrets, through one backend. A `Copy` handle: make one per store and keep it.
///
/// Each handle has its own listeners, so two engines on one store do not share a listener map.
/// Two handles on one store (the same backend, or the same platform options) run their
/// operations in order.
#[derive(Clone, Copy)]
pub struct SecureStorage {
    surface: &'static Surface,
}

/// A stop for a listener or a protected-data subscription. Calling [`Subscription::stop`] ends it.
pub struct Subscription {
    stop: Option<Box<dyn FnOnce()>>,
}

impl Subscription {
    pub(crate) fn new(stop: impl FnOnce() + 'static) -> Self {
        Self {
            stop: Some(Box::new(stop)),
        }
    }

    /// Ends the subscription. Its listener is not called again.
    pub fn stop(mut self) {
        if let Some(stop) = self.stop.take() {
            stop();
        }
    }
}

/// What one handle owns: the backend, its defaults, and its listeners.
struct Surface {
    backend: Arc<dyn Backend>,
    defaults: PlatformOptions,
    listeners: RefCell<Vec<Listener>>,
    next_listener: Cell<u64>,
}

/// What a listener is called with: the key's new value, or `None` when it was removed.
type ListenerCall = Rc<dyn Fn(Option<&str>)>;

/// A registered listener.
#[derive(Clone)]
struct Listener {
    id: u64,
    key: String,
    call: ListenerCall,
}

impl SecureStorage {
    /// A store on the platform's own secret store, with `options` as its defaults.
    pub fn new(options: SecureStorageOptions) -> Self {
        Self::with_backend(platform_backend(), options)
    }

    /// A store on `backend`, with `options` as its defaults. Tests and other platforms use this.
    ///
    /// Each handle keeps a small allocation for the life of the process, so make one per store.
    pub fn with_backend(backend: Arc<dyn Backend>, options: SecureStorageOptions) -> Self {
        let surface = Box::leak(Box::new(Surface {
            backend,
            defaults: options.platform,
            listeners: RefCell::new(Vec::new()),
            next_listener: Cell::new(0),
        }));
        Self { surface }
    }

    /// An empty store in memory, for tests and for apps that keep nothing on the device.
    pub fn memory() -> Self {
        Self::with_initial_values(BTreeMap::new())
    }

    /// A store in memory that starts with `values`, as upstream's `setMockInitialValues`.
    pub fn with_initial_values(values: BTreeMap<String, String>) -> Self {
        let backend = crate::memory::MemoryBackend::with_initial_values(values);
        Self::with_backend(Arc::new(backend), SecureStorageOptions::default())
    }

    /// Stores `value` under `key`. `None` deletes the key, as upstream does.
    pub async fn write(
        self,
        key: &str,
        value: Option<&str>,
        options: &WriteOptions,
    ) -> Result<(), SecureStorageError> {
        check_key(key)?;
        let platform = self.resolve(options);
        match value {
            Some(value) => {
                let guard = self.lock(&platform).await;
                let backend = &*self.surface.backend;
                let stored = self
                    .with_reset(&platform, || backend.write(&platform, key, value))
                    .await
                    .map_err(|e| e.with_key(key));
                drop(guard);
                stored?;
                self.notify(key, Some(value));
                Ok(())
            }
            None => self.remove(key, &platform).await,
        }
    }

    /// The value under `key`, or `None` when there is none. The value is zeroed when dropped.
    pub async fn read(
        self,
        key: &str,
        options: &ReadOptions,
    ) -> Result<Option<Zeroizing<String>>, SecureStorageError> {
        check_key(key)?;
        let platform = self.resolve(options);
        self.wait_for_writes(&platform).await;
        let backend = &*self.surface.backend;
        self.with_reset(&platform, || backend.read(&platform, key))
            .await
            .map_err(|e| e.with_key(key))
    }

    /// Whether a value is stored under `key`.
    pub async fn contains_key(
        self,
        key: &str,
        options: &ContainsKeyOptions,
    ) -> Result<bool, SecureStorageError> {
        check_key(key)?;
        let platform = self.resolve(options);
        self.wait_for_writes(&platform).await;
        let backend = &*self.surface.backend;
        self.with_reset(&platform, || backend.contains_key(&platform, key))
            .await
            .map_err(|e| e.with_key(key))
    }

    /// Removes the value under `key`. A key that is not there is not an error.
    pub async fn delete(
        self,
        key: &str,
        options: &DeleteOptions,
    ) -> Result<(), SecureStorageError> {
        check_key(key)?;
        let platform = self.resolve(options);
        self.remove(key, &platform).await
    }

    /// Every key and value of the store, in key order.
    pub async fn read_all(
        self,
        options: &ReadAllOptions,
    ) -> Result<BTreeMap<String, Zeroizing<String>>, SecureStorageError> {
        let platform = self.resolve(options);
        self.wait_for_writes(&platform).await;
        let backend = &*self.surface.backend;
        self.with_reset(&platform, || backend.read_all(&platform))
            .await
    }

    /// Removes every value of the store. Each listener is told its key is gone.
    pub async fn delete_all(self, options: &DeleteAllOptions) -> Result<(), SecureStorageError> {
        let platform = self.resolve(options);
        let guard = self.lock(&platform).await;
        let removed = caught(self.surface.backend.delete_all(&platform)).await;
        drop(guard);
        removed?;
        for listener in self.listeners_snapshot() {
            (listener.call)(None);
        }
        Ok(())
    }

    /// Calls `listener` with the new value of `key`, or `None`, after each write or delete made
    /// through this handle. The result stops it.
    pub fn register_listener(
        &self,
        key: &str,
        listener: impl Fn(Option<&str>) + 'static,
    ) -> Subscription {
        let surface = self.surface;
        let id = surface.next_listener.get();
        surface.next_listener.set(id + 1);
        surface.listeners.borrow_mut().push(Listener {
            id,
            key: key.to_owned(),
            call: Rc::new(listener),
        });
        Subscription::new(move || {
            surface.listeners.borrow_mut().retain(|l| l.id != id);
        })
    }

    /// How many listeners this handle holds now. Apps use it to find a listener that was not ended.
    pub fn listener_count(&self) -> usize {
        self.surface.listeners.borrow().len()
    }

    /// Ends every listener registered for `key`.
    pub fn unregister_all_listeners_for_key(&self, key: &str) {
        self.surface.listeners.borrow_mut().retain(|l| l.key != key);
    }

    /// Ends every listener of this handle.
    pub fn unregister_all_listeners(&self) {
        self.surface.listeners.borrow_mut().clear();
    }

    /// Whether the platform's protected data is readable now. Always `true` off iOS.
    pub fn is_protected_data_available(&self) -> bool {
        self.surface.backend.is_protected_data_available()
    }

    /// Calls `listener` with `true` or `false` when protected data becomes available or not
    /// (iOS). Off iOS it is never called.
    pub fn on_protected_data_change(&self, listener: impl Fn(bool) + 'static) -> Subscription {
        let stop = self
            .surface
            .backend
            .on_protected_data_change(Rc::new(listener));
        Subscription::new(stop)
    }

    /// The options a call uses: its own, or the store's defaults.
    pub(crate) fn resolve(&self, call: &CallOptions) -> PlatformOptions {
        call.platform
            .clone()
            .unwrap_or_else(|| self.surface.defaults.clone())
    }

    /// The listeners now registered, cloned so a listener may change the set.
    fn listeners_snapshot(&self) -> Vec<Listener> {
        self.surface.listeners.borrow().clone()
    }

    /// Calls the listeners of `key` with `value`.
    fn notify(&self, key: &str, value: Option<&str>) {
        let matching: Vec<Listener> = self
            .listeners_snapshot()
            .into_iter()
            .filter(|l| l.key == key)
            .collect();
        for listener in matching {
            (listener.call)(value);
        }
    }

    /// Runs a backend call. Only a permanent key failure triggers a reset, and only when the store
    /// opted in with `reset_on_error`. The retry runs once, after the reset, and never again.
    async fn with_reset<'a, T>(
        &self,
        platform: &'a PlatformOptions,
        call: impl Fn() -> BackendFuture<'a, Result<T, SecureStorageError>>,
    ) -> Result<T, SecureStorageError> {
        match caught(call()).await {
            Err(error)
                if error.kind() == ErrorKind::KeyInvalidated && platform.android.reset_on_error =>
            {
                caught(self.surface.backend.reset_store(platform))
                    .await
                    .map_err(|_| SecureStorageError::new(ErrorKind::ResetFailed))?;
                caught(call()).await.map_err(|error| {
                    if error.kind() == ErrorKind::KeyInvalidated {
                        SecureStorageError::new(ErrorKind::ResetFailed)
                    } else {
                        error
                    }
                })
            }
            other => other,
        }
    }

    /// Takes the store's lock for a write or a delete, so it runs after the writes before it.
    async fn lock(&self, platform: &PlatformOptions) -> OwnedMutexGuard<()> {
        let id = self.surface.backend.store_id(platform);
        store_lock(id).lock_owned().await
    }

    /// Waits for the writes queued before this read, then returns. Reads run without the lock, so
    /// a write may land while a read is in flight; the reactive read drops the older answer.
    async fn wait_for_writes(&self, platform: &PlatformOptions) {
        let _ = self.lock(platform).await;
    }

    /// Removes `key` under the lock, then tells its listeners.
    async fn remove(
        &self,
        key: &str,
        platform: &PlatformOptions,
    ) -> Result<(), SecureStorageError> {
        let guard = self.lock(platform).await;
        let removed = caught(self.surface.backend.delete(platform, key))
            .await
            .map_err(|e| e.with_key(key));
        drop(guard);
        removed?;
        self.notify(key, None);
        Ok(())
    }
}

impl std::fmt::Debug for SecureStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecureStorage")
    }
}

/// Polls one backend call, turning a panic inside it into a `Backend` error. The call's future is
/// dropped there, so the store's lock is released and the store stays usable.
async fn caught<T>(
    mut call: BackendFuture<'_, Result<T, SecureStorageError>>,
) -> Result<T, SecureStorageError> {
    std::future::poll_fn(move |cx| {
        match std::panic::catch_unwind(AssertUnwindSafe(|| call.as_mut().poll(cx))) {
            Ok(Poll::Ready(result)) => Poll::Ready(result),
            Ok(Poll::Pending) => Poll::Pending,
            Err(_) => Poll::Ready(Err(SecureStorageError::new(ErrorKind::Backend))),
        }
    })
    .await
}

/// Rejects keys a platform cannot store: empty ones and ones with a NUL character.
pub(crate) fn check_key(key: &str) -> Result<(), SecureStorageError> {
    if key.is_empty() || key.contains('\0') {
        return Err(SecureStorageError::new(ErrorKind::InvalidKey));
    }
    Ok(())
}

/// The lock of the store named `id`, shared by every handle on it. Created on first use, and
/// dropped when no operation holds it.
fn store_lock(id: String) -> Arc<AsyncMutex<()>> {
    static LOCKS: OnceLock<Mutex<HashMap<String, Weak<AsyncMutex<()>>>>> = OnceLock::new();
    let locks = LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut locks = locks
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    locks.retain(|_, lock| lock.strong_count() > 0);
    if let Some(lock) = locks.get(&id).and_then(Weak::upgrade) {
        return lock;
    }
    let lock = Arc::new(AsyncMutex::new(()));
    locks.insert(id, Arc::downgrade(&lock));
    lock
}

/// The backend of this platform. Platforms without one report `NotAvailable` for every call.
fn platform_backend() -> Arc<dyn Backend> {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        Arc::new(crate::apple::AppleBackend)
    }
    #[cfg(target_os = "android")]
    {
        Arc::new(crate::android::AndroidBackend::new())
    }
    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
    {
        Arc::new(crate::unsupported::UnsupportedBackend)
    }
}
