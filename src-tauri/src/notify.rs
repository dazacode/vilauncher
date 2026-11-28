//! How the launcher says "something changed": a flag that a small thread
//! turns into one event for the window at most every 50 ms, however many log
//! lines a build prints. The thread sleeps until something changes, so an
//! idle launcher does not wake 20 times a second to look at the flag.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::Thread;
use std::time::Duration;

/// The shortest time between two events: later changes in it wait and go out as one.
const GAP: Duration = Duration::from_millis(50);

#[derive(Clone)]
pub struct Ctx {
    dirty: Arc<AtomicBool>,
    notifier: Thread,
}

impl Ctx {
    /// `emit` runs on the notifier's own thread whenever something changed.
    pub fn new(emit: impl Fn() + Send + 'static) -> Ctx {
        let dirty = Arc::new(AtomicBool::new(false));
        let flag = dirty.clone();
        let notifier = std::thread::spawn(move || loop {
            // An unpark that came before this park makes it return at once, so no change is missed;
            // the loop also covers a park that returns for no reason.
            while !flag.swap(false, Ordering::AcqRel) {
                std::thread::park();
            }
            emit();
            std::thread::sleep(GAP);
        })
        .thread()
        .clone();
        Ctx { dirty, notifier }
    }

    pub fn request_repaint(&self) {
        // Only the first change since the last event wakes the thread.
        if !self.dirty.swap(true, Ordering::AcqRel) {
            self.notifier.unpark();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::time::Instant;

    fn counter() -> (Ctx, Arc<AtomicUsize>) {
        let count = Arc::new(AtomicUsize::new(0));
        let seen = count.clone();
        let ctx = Ctx::new(move || {
            seen.fetch_add(1, Ordering::SeqCst);
        });
        (ctx, count)
    }

    fn wait_for(count: &AtomicUsize, at_least: usize) -> bool {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(2) {
            if count.load(Ordering::SeqCst) >= at_least {
                return true;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        false
    }

    #[test]
    fn a_change_is_sent() {
        let (ctx, count) = counter();
        ctx.request_repaint();
        assert!(wait_for(&count, 1));
    }

    #[test]
    fn many_changes_at_once_are_one_or_two_events() {
        let (ctx, count) = counter();
        for _ in 0..10_000 {
            ctx.request_repaint();
        }
        assert!(wait_for(&count, 1));
        std::thread::sleep(GAP * 3);
        let sent = count.load(Ordering::SeqCst);
        assert!((1..=2).contains(&sent), "{sent} events");
    }

    #[test]
    fn nothing_changed_nothing_sent() {
        let (_ctx, count) = counter();
        std::thread::sleep(GAP * 3);
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_change_after_a_quiet_spell_is_sent_too() {
        let (ctx, count) = counter();
        ctx.request_repaint();
        assert!(wait_for(&count, 1));
        std::thread::sleep(GAP * 3);
        ctx.request_repaint();
        assert!(wait_for(&count, 2));
    }
}
