//! The reactive read, on `Headless` with a manual clock: bindings, racing reads, and owners.

use std::{
    cell::Cell,
    collections::BTreeMap,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering::SeqCst},
    },
    time::{Duration, SystemTime},
};

use nsh_core::prelude::*;
use nsh_engine::{Clock, EngineConfig, Root};
use nsh_secure_storage::{
    Backend, BackendFuture, MemoryBackend, SecureStorage, SecureStorageError, SecureValue,
    WriteOptions, use_secure_value,
};
use nsh_testing::Headless;
use zeroize::Zeroizing;

/// Runs one scenario of [`Harness`].
#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    /// Two keys, each with a binding that counts its runs.
    PerKey,
    /// One key whose first read takes a second on the manual clock.
    SlowRead,
    /// A child that reads a key, and can be hidden by its parent.
    Owner,
}

/// The props of [`Harness`]: the scenario's index, as [`Mode`] is `0`, `1` or `2`.
#[derive(Clone, PartialEq, Props)]
pub struct HarnessProps {
    pub mode: Prop<u32>,
}

impl Mode {
    fn from_index(index: u32) -> Self {
        match index {
            0 => Mode::PerKey,
            1 => Mode::SlowRead,
            _ => Mode::Owner,
        }
    }
}

static A_RUNS: AtomicUsize = AtomicUsize::new(0);
static B_RUNS: AtomicUsize = AtomicUsize::new(0);
static CHILD_RUNS: AtomicUsize = AtomicUsize::new(0);
static SEEN_A: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// A memory backend whose reads answer after a delay on the engine's clock. The answer is the
/// value as it was when the read began.
struct TimedBackend {
    inner: MemoryBackend,
    read_delay: Duration,
}

impl Backend for TimedBackend {
    fn store_id(&self, options: &nsh_secure_storage::PlatformOptions) -> String {
        self.inner.store_id(options)
    }

    fn write<'a>(
        &'a self,
        options: &'a nsh_secure_storage::PlatformOptions,
        key: &'a str,
        value: &'a str,
    ) -> BackendFuture<'a, Result<(), SecureStorageError>> {
        self.inner.write(options, key, value)
    }

    fn read<'a>(
        &'a self,
        options: &'a nsh_secure_storage::PlatformOptions,
        key: &'a str,
    ) -> BackendFuture<'a, Result<Option<Zeroizing<String>>, SecureStorageError>> {
        Box::pin(async move {
            let snapshot = self.inner.read(options, key).await;
            timer(self.read_delay).await;
            snapshot
        })
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

fn store(initial: &[(&str, &str)], read_delay: Duration) -> SecureStorage {
    let values = initial
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    let backend = TimedBackend {
        inner: MemoryBackend::with_initial_values(values),
        read_delay,
    };
    SecureStorage::with_backend(Arc::new(backend), Default::default())
}

/// The text of a value, or `none`.
fn shown(value: Option<Zeroizing<String>>) -> String {
    value.map_or_else(|| "none".to_owned(), |v| v.as_str().to_owned())
}

/// Keeps a task alive for as long as its owner, since dropping a task cancels it.
fn keep(tasks: Stored<Vec<Spawned<()>>>, task: Spawned<()>) {
    tasks.with(|held| held.push(task));
}

fn write_new(storage: SecureStorage, key: &'static str, value: &'static str) -> Spawned<()> {
    spawn(async move {
        storage
            .write(key, Some(value), &WriteOptions::default())
            .await
            .unwrap();
    })
}

#[component]
pub fn Harness(props: HarnessProps) -> impl IntoElement {
    match Mode::from_index(props.mode.get_untracked()) {
        Mode::PerKey => per_key().into_element(),
        Mode::SlowRead => slow_read().into_element(),
        Mode::Owner => owner().into_element(),
    }
}

fn per_key() -> impl IntoElement {
    let storage = store(&[("a", "old-a"), ("b", "old-b")], Duration::ZERO);
    let a = use_secure_value(storage, "a");
    let b = use_secure_value(storage, "b");
    let tasks = stored(Vec::<Spawned<()>>::new());
    view()
        .child(text(move || {
            A_RUNS.fetch_add(1, SeqCst);
            format!("a={}", shown(a.value()))
        }))
        .child(text(move || {
            B_RUNS.fetch_add(1, SeqCst);
            format!("b={}", shown(b.value()))
        }))
        .child(
            view()
                .role(AccessibilityRole::Button)
                .label("Write a")
                .on_press(move || keep(tasks, write_new(storage, "a", "new-a")))
                .child(text("Write a")),
        )
}

fn slow_read() -> impl IntoElement {
    let storage = store(&[("a", "old")], Duration::from_secs(1));
    let a = use_secure_value(storage, "a");
    let tasks = stored(Vec::<Spawned<()>>::new());
    view()
        .child(text(move || {
            let shown_now = format!("a={}", shown(a.value()));
            SEEN_A.lock().unwrap().push(shown_now.clone());
            shown_now
        }))
        .child(
            view()
                .role(AccessibilityRole::Button)
                .label("Write new")
                .on_press(move || keep(tasks, write_new(storage, "a", "new")))
                .child(text("Write new")),
        )
}

fn owner() -> impl IntoElement {
    let storage = store(&[("a", "old")], Duration::ZERO);
    let show = signal(true);
    let captured: Rc<Cell<Option<SecureValue>>> = Rc::new(Cell::new(None));
    let tasks = stored(Vec::<Spawned<()>>::new());
    let report = captured.clone();
    // Pressing Check re-runs the report after the hidden child has been disposed.
    let check = signal(0_u32);
    view()
        .child(
            view()
                .role(AccessibilityRole::Button)
                .label("Hide")
                .on_press(move || show.set(false))
                .child(text("Hide")),
        )
        .child(
            view()
                .role(AccessibilityRole::Button)
                .label("Check")
                .on_press(move || check.update(|n| *n += 1))
                .child(text("Check")),
        )
        .child(
            view()
                .role(AccessibilityRole::Button)
                .label("Write new")
                .on_press(move || keep(tasks, write_new(storage, "a", "new")))
                .child(text("Write new")),
        )
        .child(text(move || {
            let _ = check.get();
            let cleared = report
                .take()
                .map(|value| {
                    let cleared = value.value().is_none();
                    report.set(Some(value));
                    cleared
                })
                .unwrap_or(true);
            format!("cleared={cleared} listeners={}", storage.listener_count())
        }))
        .child(when(show, move || child(storage, captured.clone())))
}

fn child(storage: SecureStorage, captured: Rc<Cell<Option<SecureValue>>>) -> impl IntoElement {
    let a = use_secure_value(storage, "a");
    captured.set(Some(a));
    text(move || {
        CHILD_RUNS.fetch_add(1, SeqCst);
        format!("child a={}", shown(a.value()))
    })
}

fn start(mode: Mode) -> Headless {
    let config = EngineConfig {
        clock: Clock::manual(SystemTime::UNIX_EPOCH),
        ..EngineConfig::new(Root::new::<Harness>(move || HarnessProps {
            mode: (mode as u32).into(),
        }))
    };
    Headless::start(config).unwrap()
}

#[test]
fn a_binding_on_one_key_wakes_only_for_that_key() -> Result<(), String> {
    let mut app = start(Mode::PerKey);
    app.wait_for("a=old-a")?;
    app.wait_for("b=old-b")?;
    let b_before = B_RUNS.load(SeqCst);
    let a_before = A_RUNS.load(SeqCst);

    app.press("role:button[Write a]")?;
    app.wait_for("a=new-a")?;
    assert!(
        A_RUNS.load(SeqCst) > a_before,
        "the binding on the written key runs again"
    );
    assert_eq!(
        B_RUNS.load(SeqCst),
        b_before,
        "the binding on the other key must not run for a write to this key"
    );
    assert!(app.screen().shows("b=old-b"));
    Ok(())
}

#[test]
fn a_write_during_a_slow_read_wins() -> Result<(), String> {
    SEEN_A.lock().unwrap().clear();
    let mut app = start(Mode::SlowRead);
    app.wait_for("a=none")?;
    app.press("role:button[Write new]")?;
    app.wait_for("a=new")?;

    // The first read answers after its second on the manual clock, with the value it saw.
    app.run_for(Duration::from_secs(2))?;
    app.run_for(Duration::from_secs(1))?;

    assert!(
        app.screen().shows("a=new"),
        "the older read must not overwrite the write"
    );
    let seen = SEEN_A.lock().unwrap().clone();
    let first_new = seen.iter().position(|s| s == "a=new");
    assert!(
        first_new.is_some_and(|at| !seen[at..].iter().any(|s| s == "a=old")),
        "no reader may show the old value after the write: {seen:?}"
    );
    Ok(())
}

#[test]
fn use_secure_value_clears_when_its_owner_goes() -> Result<(), String> {
    let mut app = start(Mode::Owner);
    app.wait_for("child a=old")?;
    app.press("role:button[Hide]")?;
    app.wait_for_gone("child a=old", Duration::from_secs(5))?;
    app.press("role:button[Check]")?;
    app.wait_for("cleared=true listeners=0")?;

    let runs_after_hide = CHILD_RUNS.load(SeqCst);
    app.press("role:button[Write new]")?;
    app.run_for(Duration::from_secs(1))?;
    assert_eq!(
        CHILD_RUNS.load(SeqCst),
        runs_after_hide,
        "a gone owner's binding must not run again"
    );
    assert!(app.screen().shows("cleared=true listeners=0"));
    Ok(())
}
