use super::{app::Controller, scheduling};
use crate::navigation::Point;
use crate::routing::{Command, KeyRouter, Modifiers};
use anyhow::{Context, Result};
use objc2_core_foundation::{
    CFMachPort, CFRetained, CFRunLoop, CFRunLoopSource, kCFRunLoopCommonModes,
};
use objc2_core_graphics::{
    CGEvent, CGEventField, CGEventFlags, CGEventSource, CGEventSourceStateID, CGEventTapLocation,
    CGEventTapOptions, CGEventTapPlacement, CGEventTapProxy, CGEventType, CGMouseButton,
};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::rc::{Rc, Weak};

#[derive(Clone, Copy)]
pub enum Action {
    Command(Command),
    Failed(&'static str),
}

/// Shared dispatch stays main-thread-only and contains no AppKit objects.
pub struct Dispatch {
    router: RefCell<KeyRouter>,
    queue: RefCell<VecDeque<(u64, Action)>>,
    generation: Cell<u64>,
    pending: Cell<bool>,
    controller: Weak<Controller>,
}
impl Dispatch {
    pub fn new(controller: Weak<Controller>) -> Rc<Self> {
        Rc::new(Self {
            router: RefCell::new(KeyRouter::default()),
            queue: RefCell::new(VecDeque::new()),
            generation: Cell::new(0),
            pending: Cell::new(false),
            controller,
        })
    }
    pub fn activate(&self) {
        self.advance();
        self.router.borrow_mut().activate();
    }
    pub fn stop(&self) {
        self.router.borrow_mut().deactivate();
        self.advance();
    }
    fn advance(&self) {
        // A generation is an opaque cancellation token, never a count.
        self.generation.set(self.generation.get().wrapping_add(1));
        self.queue.borrow_mut().clear();
    }
    pub fn generation(&self) -> u64 {
        self.generation.get()
    }
    pub fn active(&self) -> bool {
        self.router.borrow().is_active()
    }
    pub fn take_actions(&self) -> VecDeque<(u64, Action)> {
        self.pending.set(false);
        std::mem::take(&mut *self.queue.borrow_mut())
    }
    fn enqueue(&self, action: Action) {
        let mut queue = self.queue.borrow_mut();
        if matches!(
            action,
            Action::Command(Command::Exit | Command::Dismiss) | Action::Failed(_)
        ) {
            // Exiting, typing, or failure cancels clicks waiting in this run-loop turn.
            queue.clear();
        }
        if queue.len() >= 128 {
            queue.clear();
            self.router.borrow_mut().deactivate();
            queue.push_back((
                self.generation.get(),
                Action::Failed("Input processing fell behind. Activate again."),
            ));
        } else {
            queue.push_back((self.generation.get(), action));
        }
        drop(queue);
        self.schedule();
    }
    fn schedule(&self) {
        if self.pending.replace(true) {
            return;
        }
        let weak = self.controller.clone();
        scheduling::defer(move || {
            if let Some(controller) = weak.upgrade() {
                controller.drain();
            }
        });
    }
}

struct CallbackContext {
    dispatch: Rc<Dispatch>,
    failed: Cell<bool>,
}

/// The stable callback allocation outlives the port and source. No callback can
/// execute after Drop removes its source and invalidates its port on main thread.
pub struct EventTap {
    port: CFRetained<CFMachPort>,
    source: CFRetained<CFRunLoopSource>,
    run_loop: CFRetained<CFRunLoop>,
    _context: Box<CallbackContext>,
}
impl EventTap {
    pub fn new(dispatch: Rc<Dispatch>) -> Result<Self> {
        let mut context = Box::new(CallbackContext {
            dispatch,
            failed: Cell::new(false),
        });
        let mask = (1_u64 << CGEventType::KeyDown.0)
            | (1_u64 << CGEventType::KeyUp.0)
            | (1_u64 << CGEventType::FlagsChanged.0);
        // SAFETY: Box allocation remains stable until tap/source invalidation.
        // Callback runs only on the main loop where this source is installed.
        let port = unsafe { CGEvent::tap_create(CGEventTapLocation::SessionEventTap,
            CGEventTapPlacement::HeadInsertEventTap, CGEventTapOptions::Default,
            mask, Some(callback), (&mut *context as *mut CallbackContext).cast()) }
            .context("macOS refused the keyboard event tap. Check Accessibility permission, then activate again")?;
        let source = CFMachPort::new_run_loop_source(None, Some(&port), 0)
            .context("could not create the keyboard run-loop source")?;
        let run_loop = CFRunLoop::main().context("the main run loop is unavailable")?;
        // SAFETY: Apple's immutable common-modes constant is valid for add_source.
        unsafe {
            run_loop.add_source(Some(&source), kCFRunLoopCommonModes);
        }
        CGEvent::tap_enable(&port, true);
        Ok(Self {
            port,
            source,
            run_loop,
            _context: context,
        })
    }
    pub fn enable(&self) {
        CGEvent::tap_enable(&self.port, true);
    }
    // The idle tap remains enabled solely for the physical Mission Control key.
    // Ordinary navigation passes through whenever the session is inactive.
}
impl Drop for EventTap {
    fn drop(&mut self) {
        CGEvent::tap_enable(&self.port, false);
        // SAFETY: Same loop/mode/source triple used for registration; context is
        // still alive while the native resources are removed and invalidated.
        unsafe {
            self.run_loop
                .remove_source(Some(&self.source), kCFRunLoopCommonModes);
        }
        self.source.invalidate();
        self.port.invalidate();
    }
}

// SAFETY: Core Graphics supplies the live event and our boxed context. Both are
// borrowed only during this callback; it never retains or releases the event.
unsafe extern "C-unwind" fn callback(
    _proxy: CGEventTapProxy,
    kind: CGEventType,
    event: NonNull<CGEvent>,
    info: *mut c_void,
) -> *mut CGEvent {
    // SAFETY: EventTap owns the non-null context until its source is invalidated.
    let context = unsafe { &*info.cast::<CallbackContext>() };
    let dispatch = &context.dispatch;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if kind == CGEventType::TapDisabledByTimeout || kind == CGEventType::TapDisabledByUserInput
        {
            if context.failed.replace(true) {
                return event.as_ptr();
            }
            dispatch.router.borrow_mut().interception_lost();
            let reason = if kind == CGEventType::TapDisabledByTimeout {
                "macOS disabled keyboard interception because its callback timed out. Choose Activate in the menu to recover."
            } else {
                "macOS disabled keyboard interception due to user input. Choose Activate in the menu to recover."
            };
            dispatch.enqueue(Action::Failed(reason));
            return event.as_ptr();
        }
        // SAFETY: The event is borrowed from Core Graphics for this invocation.
        let event_ref = unsafe { event.as_ref() };
        let code =
            CGEvent::integer_value_field(Some(event_ref), CGEventField::KeyboardEventKeycode);
        let Ok(code) = usize::try_from(code) else {
            return event.as_ptr();
        };
        let mut router = dispatch.router.borrow_mut();
        let (consume, command) = if kind == CGEventType::FlagsChanged {
            let flags = CGEvent::flags(Some(event_ref));
            let pressed = match code {
                54 | 55 => flags.contains(CGEventFlags::MaskCommand),
                58 | 61 => flags.contains(CGEventFlags::MaskAlternate),
                59 | 62 => flags.contains(CGEventFlags::MaskControl),
                63 => flags.contains(CGEventFlags::MaskSecondaryFn),
                // Caps Lock reports its toggle, not a conventional down/up pair.
                57 => true,
                56 | 60 => flags.contains(CGEventFlags::MaskShift),
                _ => false,
            };
            (false, router.modifier_changed(code, pressed))
        } else if kind == CGEventType::KeyUp {
            (router.release(code), None)
        } else if kind == CGEventType::KeyDown {
            let flags = CGEvent::flags(Some(event_ref));
            router.press(
                code,
                Modifiers {
                    shift: flags.contains(CGEventFlags::MaskShift),
                    shortcut: flags.intersects(
                        CGEventFlags::MaskCommand
                            | CGEventFlags::MaskControl
                            | CGEventFlags::MaskAlternate,
                    ),
                },
                CGEvent::integer_value_field(
                    Some(event_ref),
                    CGEventField::KeyboardEventAutorepeat,
                ) != 0,
            )
        } else {
            (false, None)
        };
        drop(router);
        if let Some(command) = command {
            dispatch.enqueue(Action::Command(command));
        }
        if consume {
            std::ptr::null_mut()
        } else {
            event.as_ptr()
        }
    }));
    match result {
        Ok(value) => value,
        Err(_) => {
            // The panic boundary prevents unwinding into the system. A fresh
            // deferred turn tears down input without displaying UI in the tap.
            if let Ok(mut router) = dispatch.router.try_borrow_mut() {
                router.deactivate();
            }
            dispatch.enqueue(Action::Failed(
                "Keyboard interception failed. Activate again.",
            ));
            event.as_ptr()
        }
    }
}

pub fn click(point: Point) -> Result<()> {
    let source = CGEventSource::new(CGEventSourceStateID::Private)
        .context("could not create a click event source")?;
    let down = CGEvent::new_mouse_event(
        Some(&source),
        CGEventType::LeftMouseDown,
        objc2_core_foundation::CGPoint::new(point.x, point.y),
        CGMouseButton::Left,
    )
    .context("could not create mouse-down")?;
    let up = CGEvent::new_mouse_event(
        Some(&source),
        CGEventType::LeftMouseUp,
        objc2_core_foundation::CGPoint::new(point.x, point.y),
        CGMouseButton::Left,
    )
    .context("could not create mouse-up")?;
    for event in [&down, &up] {
        CGEvent::set_flags(Some(event), CGEventFlags::empty());
        CGEvent::set_integer_value_field(Some(event), CGEventField::MouseEventClickState, 1);
    }
    CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&down));
    CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&up));
    Ok(())
}
