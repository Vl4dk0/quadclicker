use super::display::Display;
use crate::navigation::{NavigationMode, OverlayState, Rect};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSBackingStoreType, NSBezierPath, NSColor, NSCompositingOperation, NSFont, NSFontAttributeName,
    NSForegroundColorAttributeName, NSPanel, NSRectFillUsingOperation, NSStatusWindowLevel,
    NSStringDrawing, NSView, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{NSDictionary, NSPoint, NSRect, NSSize, NSString};
use std::cell::Cell;

// SAFETY: NSView subclass has no extra superclass invariants. Its ivar is plain
// geometry and methods run exclusively on the AppKit main thread.
define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Cell<OverlayState>]
    struct SelectionView;
    impl SelectionView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool { true }
        #[unsafe(method(drawRect:))]
        fn draw(&self, dirty: NSRect) {
            NSColor::clearColor().setFill();
            // SAFETY: AppKit invokes drawRect with a current graphics context.
            NSRectFillUsingOperation(dirty, NSCompositingOperation::Copy);
            let state = self.ivars().get();
            draw_grid(state);
            draw_selection(state.selection);
            draw_mode_badge(state);
        }
    }
);

// Dense grids would obscure the application underneath. Keep the exact selected
// cell and density badge visible while suppressing unreadable context lines.
fn draw_grid(state: OverlayState) {
    let Some(divisions) = state.grid_divisions else {
        return;
    };
    let step_x = state.root.width / f64::from(divisions);
    let step_y = state.root.height / f64::from(divisions);
    if step_x.min(step_y) < 12.0 {
        return;
    }
    let grid = NSBezierPath::bezierPath();
    for index in 1..divisions {
        let x = state.root.x + step_x * f64::from(index);
        let y = state.root.y + step_y * f64::from(index);
        grid.moveToPoint(NSPoint::new(x, state.root.y));
        grid.lineToPoint(NSPoint::new(x, state.root.y + state.root.height));
        grid.moveToPoint(NSPoint::new(state.root.x, y));
        grid.lineToPoint(NSPoint::new(state.root.x + state.root.width, y));
    }
    NSColor::colorWithSRGBRed_green_blue_alpha(0.0, 0.0, 0.0, 0.35).setStroke();
    grid.setLineWidth(2.0);
    grid.stroke();
    NSColor::colorWithSRGBRed_green_blue_alpha(1.0, 1.0, 1.0, 0.45).setStroke();
    grid.setLineWidth(0.75);
    grid.stroke();
}

fn draw_selection(rect: Rect) {
    let inset = 1.5_f64.min(rect.width / 4.0).min(rect.height / 4.0);
    let bounds = NSRect::new(
        NSPoint::new(rect.x + inset, rect.y + inset),
        NSSize::new(rect.width - inset * 2.0, rect.height - inset * 2.0),
    );
    let radius = 3.0_f64
        .min(bounds.size.width / 4.0)
        .min(bounds.size.height / 4.0);
    let outline = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(bounds, radius, radius);
    NSColor::blackColor().setStroke();
    outline.setLineWidth(4.0);
    outline.stroke();
    NSColor::whiteColor().setStroke();
    outline.setLineWidth(2.0);
    outline.stroke();

    let center = rect.midpoint();
    let dot = NSBezierPath::bezierPathWithOvalInRect(NSRect::new(
        NSPoint::new(center.x - 4.0, center.y - 4.0),
        NSSize::new(8.0, 8.0),
    ));
    // A two-tone halo preserves the target's contrast even over red content.
    NSColor::blackColor().setStroke();
    dot.setLineWidth(4.0);
    dot.stroke();
    NSColor::whiteColor().setStroke();
    dot.setLineWidth(2.0);
    dot.stroke();
    NSColor::colorWithSRGBRed_green_blue_alpha(1.0, 0.18, 0.22, 1.0).setFill();
    dot.fill();
}

fn draw_mode_badge(state: OverlayState) {
    let label = match state.mode {
        NavigationMode::Quadtree => String::from("Quadtree"),
        NavigationMode::Grid => {
            let divisions = state.grid_divisions.unwrap_or(4);
            format!("Grid · {divisions}×{divisions}")
        }
    };
    let text = NSString::from_str(&label);
    let font = NSFont::systemFontOfSize(13.0);
    let color = NSColor::whiteColor();
    // SAFETY: AppKit's attribute constants are immutable. Each value has the
    // documented type: NSFont for the font and NSColor for foreground color.
    let attributes = unsafe {
        NSDictionary::from_slices(
            &[NSFontAttributeName, NSForegroundColorAttributeName],
            &[&*font as &AnyObject, &*color as &AnyObject],
        )
    };
    // SAFETY: The dictionary contains valid NSString drawing attributes.
    let text_size = unsafe { text.sizeWithAttributes(Some(&attributes)) };
    let badge = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(
        NSRect::new(
            NSPoint::new(12.0, 12.0),
            NSSize::new(text_size.width + 20.0, text_size.height + 12.0),
        ),
        6.0,
        6.0,
    );
    NSColor::colorWithSRGBRed_green_blue_alpha(0.08, 0.08, 0.09, 0.9).setFill();
    badge.fill();
    NSColor::colorWithSRGBRed_green_blue_alpha(1.0, 1.0, 1.0, 0.35).setStroke();
    badge.setLineWidth(1.0);
    badge.stroke();
    // SAFETY: drawRect supplies a current context and attributes have valid types.
    unsafe { text.drawAtPoint_withAttributes(NSPoint::new(22.0, 18.0), Some(&attributes)) };
}

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
    pub fn new(mtm: MainThreadMarker, display: Display, state: OverlayState) -> Self {
        let allocated = SelectionView::alloc(mtm).set_ivars(Cell::new(state));
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
    pub fn update(&self, state: OverlayState) {
        self.view.ivars().set(state);
        self.view.setNeedsDisplay(true);
    }
}
impl Drop for Overlay {
    fn drop(&mut self) {
        self.panel.orderOut(None);
    }
}
