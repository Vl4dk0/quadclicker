use block2::RcBlock;
use objc2_core_foundation::{CFRunLoop, kCFRunLoopCommonModes};

/// The main run loop owns a copied block; callers capture weak application owners.
pub fn defer(work: impl Fn() + 'static) {
    let Some(run_loop) = CFRunLoop::main() else {
        return;
    };
    let block = RcBlock::new(move || {
        // Keep unwinding inside Rust even when Core Foundation invokes this block.
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(&work)).is_err() {
            eprintln!("QuadClicker: deferred native callback panicked");
        }
    });
    // SAFETY: Main run loop execution serializes all AppKit access. The global
    // common-modes string is a CFString accepted by CFRunLoopPerformBlock, which
    // copies the block before returning.
    unsafe {
        run_loop.perform_block(
            kCFRunLoopCommonModes.map(|mode| mode.as_ref()),
            Some(&block),
        );
    }
    run_loop.wake_up();
}

/// A grid gesture owns one cadence, independent of which physical key macOS repeats.
pub struct RepeatTimer {
    timer: objc2_core_foundation::CFRetained<objc2_core_foundation::CFRunLoopTimer>,
}
impl RepeatTimer {
    pub fn new(delay: f64, interval: f64, work: impl Fn() + 'static) -> Option<Self> {
        use objc2_core_foundation::{CFAbsoluteTimeGetCurrent, CFRunLoopTimer};
        if !delay.is_finite() || delay <= 0.0 || !interval.is_finite() || interval <= 0.0 {
            return None;
        }
        let run_loop = CFRunLoop::main()?;
        let block = RcBlock::new(move |_| {
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(&work)).is_err() {
                eprintln!("QuadClicker: movement timer callback panicked");
            }
        });
        // SAFETY: The default allocator is valid; Core Foundation copies the block.
        // Work captures only a weak owner and executes on the registered main loop.
        let timer = unsafe {
            CFRunLoopTimer::with_handler(
                None,
                CFAbsoluteTimeGetCurrent() + delay,
                interval,
                0,
                0,
                Some(&block),
            )
        }?;
        // SAFETY: Apple's immutable common-modes constant is valid for registration.
        unsafe {
            run_loop.add_timer(Some(&timer), kCFRunLoopCommonModes);
        }
        Some(Self { timer })
    }
}
impl Drop for RepeatTimer {
    fn drop(&mut self) {
        self.timer.invalidate();
    }
}
