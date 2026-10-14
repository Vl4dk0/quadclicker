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
