//! A value of one key, read reactively: its own signal, updated by the storage's writes.

use std::{fmt, rc::Rc};

use nsh_core::prelude::*;
use zeroize::Zeroizing;

use crate::{
    error::SecureStorageError,
    options::ReadOptions,
    store::{SecureStorage, Subscription},
};

/// Where a [`SecureValue`] stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecureValueState {
    /// The first read has not answered yet.
    Loading,
    /// The value is current: read, written or deleted through the storage.
    Ready,
    /// The last read failed. [`SecureValue::error`] says why.
    Error,
}

/// A value held in a signal. It is zeroed when the signal is dropped.
#[derive(Clone)]
struct SecretValue(Zeroizing<String>);

impl PartialEq for SecretValue {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretValue(..)")
    }
}

/// The reactive value of one key. A `Copy` handle: bindings may hold it freely.
///
/// It follows writes and deletes made through its storage. A binding on one key runs again only
/// when that key changes. Its value is cleared when the owner that made it goes.
#[derive(Clone, Copy)]
pub struct SecureValue {
    key: Stored<Rc<str>>,
    value: Signal<Option<SecretValue>>,
    state: Signal<SecureValueState>,
    error: Signal<Option<SecureStorageError>>,
    refresh: Stored<Rc<dyn Fn()>>,
}

/// Reads `key` from `storage` into a reactive value owned by the current owner.
///
/// The first read starts at once. A write or delete through `storage` replaces the value without
/// a read. A read that was in flight when a write landed is dropped, so an older answer never
/// overwrites a newer write.
pub fn use_secure_value(storage: SecureStorage, key: impl Into<String>) -> SecureValue {
    let key: Rc<str> = Rc::from(key.into());
    let owner = Owner::current();
    let value = signal(None::<SecretValue>);
    let state = signal(SecureValueState::Loading);
    let error = signal(None::<SecureStorageError>);
    // Bumped by every write, delete and refresh. A read answers only if nothing bumped it since it started.
    let revision = stored(0_u64);

    // Dropping a task cancels it, so the newest read replaces and cancels the one in flight.
    let inflight = stored(None::<Spawned<()>>);
    let read_key = key.clone();
    let start_read: Rc<dyn Fn()> = Rc::new(move || {
        let read_key = read_key.clone();
        let started = revision.get() + 1;
        revision.replace(started);
        let task = owner.run(|| {
            spawn(async move {
                let answer = storage.read(&read_key, &ReadOptions::default()).await;
                apply_read(revision, started, value, state, error, answer);
            })
        });
        drop(inflight.replace(Some(task)));
    });

    let listener_key = key.clone();
    let subscription: Subscription = storage.register_listener(&listener_key, move |new| {
        revision.with(|r| *r += 1);
        batch(|| {
            value.set(new.map(|v| SecretValue(Zeroizing::new(v.to_owned()))));
            state.set(SecureValueState::Ready);
            error.set(None);
        });
    });
    cleanup(move || subscription.stop());

    let refresh = stored(start_read.clone());
    start_read();
    SecureValue {
        key: stored(key),
        value,
        state,
        error,
        refresh,
    }
}

/// Applies a read's answer, unless a write or refresh came after the read started.
fn apply_read(
    revision: Stored<u64>,
    started: u64,
    value: Signal<Option<SecretValue>>,
    state: Signal<SecureValueState>,
    error: Signal<Option<SecureStorageError>>,
    answer: Result<Option<Zeroizing<String>>, SecureStorageError>,
) {
    if revision.get() != started {
        return;
    }
    batch(|| match answer {
        Ok(found) => {
            value.set(found.map(SecretValue));
            state.set(SecureValueState::Ready);
            error.set(None);
        }
        Err(e) => {
            value.set(None);
            state.set(SecureValueState::Error);
            error.set(Some(e));
        }
    });
}

impl SecureValue {
    /// The value now, a copy the caller owns and should drop when done. `None` when the key is
    /// absent, not read yet, or the owner is gone.
    pub fn value(&self) -> Option<Zeroizing<String>> {
        self.value
            .try_get()
            .flatten()
            .map(|secret| secret.0.clone())
    }

    /// Whether the value is loading, current, or in error.
    pub fn state(&self) -> SecureValueState {
        self.state.try_get().unwrap_or(SecureValueState::Loading)
    }

    /// The error of the last read, when it failed.
    pub fn error(&self) -> Option<SecureStorageError> {
        self.error.try_get().flatten()
    }

    /// Reads the key again. A read still in flight is dropped.
    pub fn refresh(&self) {
        let start = self.refresh.get();
        start();
    }

    /// The key this value reads.
    pub fn key(&self) -> Rc<str> {
        self.key.get()
    }
}

impl fmt::Debug for SecureValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecureValue({:?})", self.key())
    }
}
