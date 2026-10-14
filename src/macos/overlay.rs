use super::display::Display;
use crate::navigation::Rect;
use objc2::rc::Retained;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSBackingStoreType, NSBezierPath, NSColor, NSCompositingOperation, NSPanel,
    NSRectFillUsingOperation, NSStatusWindowLevel, NSView, NSWindowCollectionBehavior,
    NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use std::cell::Cell;

// SAFETY: NSView subclass has no extra superclass invariants. Its ivar is plain
// geometry and methods run exclusively on the AppKit main thread.
define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Cell<Rect>]
    struct SelectionView;
    impl SelectionView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool { true }
        #[unsafe(method(drawRect:))]
        fn draw(&self, dirty: NSRect) {
            NSColor::clearColor().setFill();
            // SAFETY: AppKit invokes drawRect with a current graphics context.
            NSRectFillUsingOperation(dirty, NSCompositingOperation::Copy);
            let rect = self.ivars().get();
            let bounds = NSRect::new(NSPoint::new(rect.x + 1.5, rect.y + 1.5),
                NSSize::new((rect.width - 3.0).max(0.0), (rect.height - 3.0).max(0.0)));
            let outline = NSBezierPath::bezierPathWithRect(bounds);
            NSColor::blackColor().setStroke(); outline.setLineWidth(4.0); outline.stroke();
            NSColor::whiteColor().setStroke(); outline.setLineWidth(2.0); outline.stroke();
            let center = rect.midpoint();
            let dot = NSBezierPath::bezierPathWithOvalInRect(NSRect::new(
                NSPoint::new(center.x - 4.0, center.y - 4.0), NSSize::new(8.0, 8.0)));
            NSColor::blackColor().setFill(); dot.fill();
            NSColor::whiteColor().setStroke(); dot.setLineWidth(2.0); dot.stroke();
        }
    }
);

// SAFETY: A nonactivating NSPanel uses ordinary NSPanel ownership. Explicitly
// refusing key/main status keeps the target application focused after clicks.
define_class!(
    #[unsafe(super(NSPanel))]
    #[thread_kind = MainThreadOnly]
    struct OverlayPanel;
    impl OverlayPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key(&self) -> bool { false }
        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main(&self) -> bool { false }
    }
);

pub struct Overlay {
    panel: Retained<OverlayPanel>,
    view: Retained<SelectionView>,
}
impl Overlay {
    pub fn new(mtm: MainThreadMarker, display: Display, selection: Rect) -> Self {
        let allocated = SelectionView::alloc(mtm).set_ivars(Cell::new(selection));
        // SAFETY: NSView initWithFrame's signature matches the supplied NSRect;
        // the subclass ivars are initialized before its superclass initializer.
        let view: Retained<SelectionView> = unsafe {
            msg_send![super(allocated), initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), display.appkit.size)]
        };
        // SAFETY: The retained panel disables AppKit's release-on-close behavior
        // below; it is owned for its whole lifetime and created on main thread.
        let allocated = OverlayPanel::alloc(mtm).set_ivars(());
        // SAFETY: NSPanel's designated initializer has this exact signature;
        // Rust retains ownership and disables release-on-close immediately.
        let panel: Retained<OverlayPanel> = unsafe {
            msg_send![super(allocated),
                initWithContentRect: display.appkit,
                styleMask: NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
                backing: NSBackingStoreType::Buffered,
                defer: false]
        };
        // SAFETY: Rust owns the retained panel rather than a window controller.
        unsafe { panel.setReleasedWhenClosed(false) };
        panel.setOpaque(false);
        panel.setBackgroundColor(Some(&NSColor::clearColor()));
        panel.setHasShadow(false);
        panel.setIgnoresMouseEvents(true);
        panel.setHidesOnDeactivate(false);
        panel.setLevel(NSStatusWindowLevel);
        panel.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
        panel.setContentView(Some(&view));
        Self { panel, view }
    }
    pub fn show(&self) {
        self.panel.orderFrontRegardless();
    }
    pub fn update(&self, rect: Rect) {
        self.view.ivars().set(rect);
        self.view.setNeedsDisplay(true);
    }
}
impl Drop for Overlay {
    fn drop(&mut self) {
        self.panel.orderOut(None);
    }
}
