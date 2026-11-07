//! IndexedDB. One database per store, with two object stores: `keys` holds the store's WebCrypto
//! key under the store's name, and `values` holds one record per key under `[store, key]`, so one
//! key range covers a store and nothing else.

use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use js_sys::{Array, Uint8Array};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{
    CryptoKey, Event, IdbDatabase, IdbKeyRange, IdbObjectStore, IdbRequest, IdbTransactionMode,
};

use super::{backend_error, unavailable};
use crate::error::SecureStorageError;

const KEYS: &str = "keys";
const VALUES: &str = "values";
const VERSION: u32 = 1;
/// The error name a `put`-if-absent gives when the key is already there.
const CONSTRAINT_ERROR: &str = "ConstraintError";

/// An open connection to one store's database. It closes when dropped.
pub(super) struct Db(IdbDatabase);

impl Drop for Db {
    fn drop(&mut self) {
        self.0.close();
    }
}

impl Db {
    /// Opens (and, the first time, creates) the database `name`.
    pub(super) async fn open(name: &str) -> Result<Self, SecureStorageError> {
        let factory = web_sys::window()
            .and_then(|window| window.indexed_db().ok().flatten())
            .ok_or_else(unavailable)?;
        let request = factory
            .open_with_u32(name, VERSION)
            .map_err(|_| backend_error())?;
        let base: IdbRequest = request.clone().into();
        let upgrade = {
            let base = base.clone();
            Closure::<dyn FnMut()>::new(move || create_stores(&base))
        };
        request.set_onupgradeneeded(Some(upgrade.as_ref().unchecked_ref()));
        let result = Pending::new(base).await;
        request.set_onupgradeneeded(None);
        drop(upgrade);
        let db = result
            .map_err(|_| backend_error())?
            .dyn_into::<IdbDatabase>()
            .map_err(|_| backend_error())?;
        Ok(Self(db))
    }

    /// The store's key, or `None` when none was created yet.
    pub(super) async fn get_key(
        &self,
        store: &str,
    ) -> Result<Option<CryptoKey>, SecureStorageError> {
        let value = self
            .run(KEYS, IdbTransactionMode::Readonly, |os| {
                os.get(&JsValue::from_str(store))
            })
            .await?;
        if value.is_undefined() {
            return Ok(None);
        }
        value
            .dyn_into::<CryptoKey>()
            .map(Some)
            .map_err(|_| backend_error())
    }

    /// Stores `key` as the store's key, unless one is already there. Returns whether this call
    /// stored it. The check and the store are one IndexedDB request, so two tabs cannot both win.
    pub(super) async fn add_key(
        &self,
        store: &str,
        key: &CryptoKey,
    ) -> Result<bool, SecureStorageError> {
        let value: &JsValue = key.as_ref();
        match self
            .named(KEYS, IdbTransactionMode::Readwrite, |os| {
                os.add_with_key(value, &JsValue::from_str(store))
            })
            .await
        {
            Ok(_) => Ok(true),
            Err(name) if name == CONSTRAINT_ERROR => Ok(false),
            Err(_) => Err(backend_error()),
        }
    }

    /// The record under `key`, or `None` when there is none.
    pub(super) async fn get_entry(
        &self,
        store: &str,
        key: &str,
    ) -> Result<Option<Vec<u8>>, SecureStorageError> {
        let entry = entry_key(store, key);
        let value = self
            .run(VALUES, IdbTransactionMode::Readonly, |os| os.get(&entry))
            .await?;
        if value.is_undefined() {
            return Ok(None);
        }
        Ok(Some(Uint8Array::new(&value).to_vec()))
    }

    /// Replaces the record under `key`.
    pub(super) async fn put_entry(
        &self,
        store: &str,
        key: &str,
        record: &[u8],
    ) -> Result<(), SecureStorageError> {
        let entry = entry_key(store, key);
        let value: JsValue = Uint8Array::from(record).into();
        self.run(VALUES, IdbTransactionMode::Readwrite, |os| {
            os.put_with_key(&value, &entry)
        })
        .await
        .map(drop)
    }

    /// Removes the record under `key`. A key that is not there is not an error.
    pub(super) async fn delete_entry(
        &self,
        store: &str,
        key: &str,
    ) -> Result<(), SecureStorageError> {
        let entry = entry_key(store, key);
        self.run(VALUES, IdbTransactionMode::Readwrite, |os| {
            os.delete(&entry)
        })
        .await
        .map(drop)
    }

    /// Every record of the store, as `(key, record)`, in key order.
    pub(super) async fn entries(
        &self,
        store: &str,
    ) -> Result<Vec<(String, Vec<u8>)>, SecureStorageError> {
        let range = store_range(store)?;
        let transaction = self
            .0
            .transaction_with_str_and_mode(VALUES, IdbTransactionMode::Readonly)
            .map_err(|_| backend_error())?;
        let os = transaction
            .object_store(VALUES)
            .map_err(|_| backend_error())?;
        // Both requests are queued before either is awaited, so they read one snapshot.
        let names = os
            .get_all_keys_with_key(&range)
            .map_err(|_| backend_error())?;
        let records = os.get_all_with_key(&range).map_err(|_| backend_error())?;
        let names = Pending::new(names).await.map_err(|_| backend_error())?;
        let records = Pending::new(records).await.map_err(|_| backend_error())?;
        let names = names.unchecked_into::<Array>();
        let records = records.unchecked_into::<Array>();
        let mut entries = Vec::with_capacity(names.length() as usize);
        for (index, name) in names.iter().enumerate() {
            let key = Array::from(&name)
                .get(1)
                .as_string()
                .ok_or_else(backend_error)?;
            let record = Uint8Array::new(&records.get(index as u32)).to_vec();
            entries.push((key, record));
        }
        Ok(entries)
    }

    /// Removes every record of the store, and nothing of another store.
    pub(super) async fn delete_entries(&self, store: &str) -> Result<(), SecureStorageError> {
        let range = store_range(store)?;
        self.run(VALUES, IdbTransactionMode::Readwrite, |os| {
            os.delete(&range)
        })
        .await
        .map(drop)
    }

    /// Runs one request in a transaction of its own.
    async fn run(
        &self,
        store_name: &str,
        mode: IdbTransactionMode,
        request: impl FnOnce(&IdbObjectStore) -> Result<IdbRequest, JsValue>,
    ) -> Result<JsValue, SecureStorageError> {
        self.named(store_name, mode, request)
            .await
            .map_err(|_| backend_error())
    }

    /// Like `run`, but the error is the browser's error name, for the one caller that acts on it.
    async fn named(
        &self,
        store_name: &str,
        mode: IdbTransactionMode,
        request: impl FnOnce(&IdbObjectStore) -> Result<IdbRequest, JsValue>,
    ) -> Result<JsValue, String> {
        let transaction = self
            .0
            .transaction_with_str_and_mode(store_name, mode)
            .map_err(|_| String::new())?;
        let os = transaction
            .object_store(store_name)
            .map_err(|_| String::new())?;
        let request = request(&os).map_err(|_| String::new())?;
        Pending::new(request).await
    }
}

/// Creates the two object stores, if this is the database's first version.
fn create_stores(request: &IdbRequest) {
    let Ok(result) = request.result() else {
        return;
    };
    let Ok(db) = result.dyn_into::<IdbDatabase>() else {
        return;
    };
    let names = db.object_store_names();
    for name in [KEYS, VALUES] {
        if !names.contains(name) {
            let _ = db.create_object_store(name);
        }
    }
}

/// The array key of one record: `[store, key]`.
fn entry_key(store: &str, key: &str) -> JsValue {
    Array::of2(&JsValue::from_str(store), &JsValue::from_str(key)).into()
}

/// The key range of every record of `store`: `[store]` up to `[store, []]`, which sorts after
/// every `[store, text]`, and before the next store's keys.
fn store_range(store: &str) -> Result<JsValue, SecureStorageError> {
    let lower: JsValue = Array::of1(&JsValue::from_str(store)).into();
    let upper: JsValue = Array::of2(&JsValue::from_str(store), &Array::new()).into();
    IdbKeyRange::bound(&lower, &upper)
        .map(Into::into)
        .map_err(|_| backend_error())
}

/// An IndexedDB request as a future. Its callbacks live exactly as long as the future: they are
/// removed when it completes or is dropped, so nothing leaks and no callback runs after its drop.
struct Pending {
    request: IdbRequest,
    shared: Rc<RefCell<Shared>>,
    _on_success: Closure<dyn FnMut(Event)>,
    _on_error: Closure<dyn FnMut(Event)>,
}

#[derive(Default)]
struct Shared {
    done: Option<Result<JsValue, String>>,
    waker: Option<Waker>,
}

impl Pending {
    fn new(request: IdbRequest) -> Self {
        let shared = Rc::new(RefCell::new(Shared::default()));
        let on_success = {
            let shared = shared.clone();
            let request = request.clone();
            Closure::<dyn FnMut(Event)>::new(move |_: Event| {
                finish(&shared, request.result().map_err(|_| String::new()));
            })
        };
        let on_error = {
            let shared = shared.clone();
            let request = request.clone();
            Closure::<dyn FnMut(Event)>::new(move |event: Event| {
                // Keeps the transaction alive, so a failed add does not abort the other requests
                // of its transaction. Every transaction here holds one request, or two reads.
                event.prevent_default();
                let name = request
                    .error()
                    .ok()
                    .flatten()
                    .map(|error| error.name())
                    .unwrap_or_default();
                finish(&shared, Err(name));
            })
        };
        request.set_onsuccess(Some(on_success.as_ref().unchecked_ref()));
        request.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        Self {
            request,
            shared,
            _on_success: on_success,
            _on_error: on_error,
        }
    }
}

fn finish(shared: &RefCell<Shared>, result: Result<JsValue, String>) {
    let mut shared = shared.borrow_mut();
    shared.done = Some(result);
    if let Some(waker) = shared.waker.take() {
        waker.wake();
    }
}

impl Future for Pending {
    type Output = Result<JsValue, String>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut shared = self.shared.borrow_mut();
        match shared.done.take() {
            Some(result) => Poll::Ready(result),
            None => {
                shared.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        self.request.set_onsuccess(None);
        self.request.set_onerror(None);
    }
}
