//! iOS protected data: whether the device's data protection keys are available, and the change
//! notifications UIKit posts. macOS has no such switch, so the backend reports it only on iOS.

use std::{ptr::NonNull, rc::Rc};

use block2::RcBlock;
use objc2::{MainThreadMarker, runtime::AnyObject};
use objc2_foundation::{NSNotification, NSNotificationCenter};
use objc2_ui_kit::{
    UIApplication, UIApplicationProtectedDataDidBecomeAvailable,
    UIApplicationProtectedDataWillBecomeUnavailable,
};

/// Whether protected data is readable now. Read off the main thread, the answer is reported as
/// `false`: UIKit answers only there, and a caller must not read a store it cannot yet reach.
pub(super) fn is_available() -> bool {
    MainThreadMarker::new()
        .is_some_and(|mtm| UIApplication::sharedApplication(mtm).isProtectedDataAvailable())
}

/// Calls `listener` with `true` and `false` as protected data comes and goes. The returned
/// function removes both observers.
pub(super) fn subscribe(listener: Rc<dyn Fn(bool)>) -> Box<dyn FnOnce()> {
    let center = NSNotificationCenter::defaultCenter();
    let available = {
        let listener = listener.clone();
        RcBlock::new(move |_: NonNull<NSNotification>| listener(true))
    };
    let unavailable = RcBlock::new(move |_: NonNull<NSNotification>| listener(false));
    // SAFETY: the blocks are kept alive by the observers until they are removed in the stop below.
    let observers = unsafe {
        vec![
            center.addObserverForName_object_queue_usingBlock(
                Some(UIApplicationProtectedDataDidBecomeAvailable),
                None,
                None,
                &available,
            ),
            center.addObserverForName_object_queue_usingBlock(
                Some(UIApplicationProtectedDataWillBecomeUnavailable),
                None,
                None,
                &unavailable,
            ),
        ]
    };
    Box::new(move || {
        for observer in observers {
            let observer: &AnyObject = observer.as_ref();
            // SAFETY: the observer came from `addObserverForName` on this center and is removed once.
            unsafe { center.removeObserver(observer) };
        }
    })
}
