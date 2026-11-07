//! Keeps Keychain calls from showing a system dialog. `AuthenticationUi::Fail` and `Skip` need
//! it: on a file keychain the system ignores `kSecUseAuthenticationUI` and still asks, so the
//! interaction setting is turned off for the length of the call instead.

use std::sync::Mutex;

use security_framework::os::macos::keychain::SecKeychain;

/// While at least one guard lives, interaction is off. The state saved before the first guard is
/// restored when the last one drops.
static GUARDS: Mutex<(usize, bool)> = Mutex::new((0, false));

/// Turns keychain interaction off until dropped. macOS only; on iOS the flag is honored natively.
pub(crate) struct NoPrompt;

impl NoPrompt {
    pub(crate) fn enter() -> Self {
        let mut guards = GUARDS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guards.0 == 0 {
            guards.1 = SecKeychain::user_interaction_allowed().unwrap_or(true);
            set_interaction(false);
        }
        guards.0 += 1;
        Self
    }
}

impl Drop for NoPrompt {
    fn drop(&mut self) {
        let mut guards = GUARDS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guards.0 -= 1;
        if guards.0 == 0 && guards.1 {
            set_interaction(true);
        }
    }
}

fn set_interaction(allowed: bool) {
    // SAFETY: the call takes a plain flag and changes only this process's keychain setting.
    unsafe {
        security_framework_sys::keychain::SecKeychainSetUserInteractionAllowed(u8::from(allowed));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};

    use security_framework::os::macos::keychain::SecKeychain;

    use super::NoPrompt;

    /// Overlapping guards on several threads: while any guard lives interaction is off, and once
    /// they all drop the setting is the one saved before the first.
    #[test]
    fn concurrent_no_prompt_guards_restore_interaction() {
        let before = SecKeychain::user_interaction_allowed().expect("the state is readable");
        let barrier = Arc::new(Barrier::new(8));
        let workers: Vec<_> = (0..8)
            .map(|_| {
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    for _ in 0..200 {
                        let guard = NoPrompt::enter();
                        assert!(
                            !SecKeychain::user_interaction_allowed().unwrap_or(true),
                            "interaction is off while a guard lives"
                        );
                        drop(guard);
                    }
                })
            })
            .collect();
        for worker in workers {
            worker.join().expect("a worker finishes");
        }
        assert_eq!(
            SecKeychain::user_interaction_allowed().expect("the state is readable"),
            before,
            "the setting is restored after the last guard"
        );
    }
}
