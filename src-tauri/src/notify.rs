//! How the launcher says "something changed": a flag that a small thread
//! turns into one event for the window at most every 50 ms, however many log
//! lines a build prints.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct Ctx {
    dirty: Arc<AtomicBool>,
}

impl Ctx {
    /// `emit` runs on the notifier's own thread whenever something changed.
    pub fn new(emit: impl Fn() + Send + 'static) -> Ctx {
        let dirty = Arc::new(AtomicBool::new(false));
        let flag = dirty.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(50));
            if flag.swap(false, Ordering::AcqRel) {
                emit();
            }
        });
        Ctx { dirty }
    }

    pub fn request_repaint(&self) {
        self.dirty.store(true, Ordering::Release);
    }
}
