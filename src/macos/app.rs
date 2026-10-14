use super::{
    display::Display,
    input::{self, Action, Dispatch, EventTap},
    overlay::Overlay,
    permissions, scheduling,
};
use crate::navigation::Selection;
use crate::routing::Command;
use anyhow::{Context, Result};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSMenu, NSMenuDelegate,
    NSMenuItem, NSStatusBar, NSStatusItem, NSVariableStatusItemLength,
};
use objc2_foundation::{
    NSNotification, NSNotificationCenter, NSObject, NSObjectProtocol, ns_string,
};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

struct Session {
    display: Display,
    selection: Selection,
    overlay: Overlay,
}

/// Application policy owns a single selection session; native callbacks only
/// request transitions, never borrow this state from the keyboard callback.
pub(super) struct Controller {
    mtm: MainThreadMarker,
    dispatch: Rc<Dispatch>,
    tap: RefCell<Option<EventTap>>,
    session: RefCell<Option<Session>>,
    menu_open: Cell<bool>,
    activation_requested: Cell<bool>,
    shutting_down: Cell<bool>,
    presenting: Cell<bool>,
}
impl Controller {
    fn new(mtm: MainThreadMarker) -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            mtm,
            dispatch: Dispatch::new(weak.clone()),
            tap: RefCell::new(None),
            session: RefCell::new(None),
            menu_open: Cell::new(false),
            activation_requested: Cell::new(false),
            shutting_down: Cell::new(false),
            presenting: Cell::new(false),
        })
    }
    fn launch(self: &Rc<Self>) {
        if permissions::granted() {
            self.request_activation();
        } else {
            // Request consent once at launch, without stacking our own dialog
            // over the system prompt or redirecting the user into Settings.
            permissions::request_on_launch();
        }
    }
    fn request_activation(self: &Rc<Self>) {
        if self.shutting_down.get() {
            return;
        }
        if self.menu_open.get() {
            self.activation_requested.set(true);
            return;
        }
        let weak = Rc::downgrade(self);
        scheduling::defer(move || {
            if let Some(controller) = weak.upgrade() {
                controller.activate();
            }
        });
    }
    fn activate(&self) {
        if self.shutting_down.get() || self.menu_open.get() || self.presenting.get() {
            return;
        }
        self.stop();
        if !permissions::granted() {
            self.show_permission_help();
            return;
        }
        if let Err(error) = self.start_session() {
            self.stop();
            self.show_error(&format!("{error:#}"));
        }
    }
    fn start_session(&self) -> Result<()> {
        let display = Display::under_pointer(self.mtm)?;
        let selection = Selection::new(display.local_bounds()?);
        let overlay = Overlay::new(self.mtm, display, selection.current());
        if self.tap.borrow().is_none() {
            let tap = EventTap::new(self.dispatch.clone())?;
            *self.tap.borrow_mut() = Some(tap);
        }
        self.dispatch.activate();
        if let Some(tap) = self.tap.borrow().as_ref() {
            tap.enable();
        }
        overlay.show();
        *self.session.borrow_mut() = Some(Session {
            display,
            selection,
            overlay,
        });
        Ok(())
    }
    fn stop(&self) {
        self.dispatch.stop();
        // Drop after releasing the borrow: ordering a native window out can
        // deliver AppKit notifications synchronously.
        let session = self.session.borrow_mut().take();
        drop(session);
    }
    pub(super) fn drain(&self) {
        let actions = self.dispatch.take_actions();
        for (generation, action) in actions {
            if generation != self.dispatch.generation() {
                continue;
            }
            match action {
                Action::Command(Command::Activate) => {
                    eprintln!(
                        "QuadClicker: physical Mission Control activation received (keycode 160)"
                    );
                    self.activate();
                }
                Action::Command(Command::Exit | Command::Dismiss) => self.stop(),
                Action::Failed(message) => {
                    eprintln!("QuadClicker: {message}");
                    self.stop();
                    let failed_tap = self.tap.borrow_mut().take();
                    drop(failed_tap);
                    self.show_error(message);
                    break;
                }
                Action::Command(command) => {
                    if !self.dispatch.active() {
                        continue;
                    }
                    if let Err(error) = self.command(command) {
                        self.stop();
                        self.show_error(&format!("{error:#}"));
                        break;
                    }
                }
            }
        }
    }
    fn command(&self, command: Command) -> Result<()> {
        // Validate permission and display immediately before every click so a
        // stale session cannot send a click to a reconfigured desktop.
        if command == Command::Click && !permissions::granted() {
            anyhow::bail!(
                "Accessibility permission was revoked. Enable QuadClicker in System Settings and activate again."
            );
        }
        let mut session_slot = self.session.borrow_mut();
        let Some(session) = session_slot.as_mut() else {
            return Ok(());
        };
        if !session.display.unchanged(self.mtm) {
            anyhow::bail!(
                "The display configuration changed. Activate again to select the current display."
            );
        }
        match command {
            Command::Descend(quadrant) => session.selection.descend(quadrant),
            Command::Back => session.selection.back(),
            Command::Reset => session.selection.reset(),
            Command::Click => {
                return input::click(
                    session
                        .display
                        .click_point(session.selection.current().midpoint()),
                );
            }
            Command::Exit | Command::Dismiss | Command::Activate => return Ok(()),
        }
        session.overlay.update(session.selection.current());
        Ok(())
    }
    fn show_permission_help(&self) {
        self.present(|| permissions::explain_missing_permission(self.mtm));
    }
    fn show_error(&self, message: &str) {
        self.present(|| permissions::error(self.mtm, message));
    }
    fn present(&self, show: impl FnOnce()) {
        if self.presenting.replace(true) {
            return;
        }
        // AppKit's modal run loop can deliver deferred activation blocks. The
        // scoped guard keeps input inactive throughout nested native delivery.
        struct PresentationGuard<'a>(&'a Cell<bool>);
        impl Drop for PresentationGuard<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _guard = PresentationGuard(&self.presenting);
        self.activation_requested.set(false);
        self.stop();
        show();
    }
    fn shutdown(&self) {
        self.shutting_down.set(true);
        self.stop();
        let tap = self.tap.borrow_mut().take();
        drop(tap);
    }
}

// SAFETY: NSObject has no subclass invariants; this delegate is main-thread-only.
// All registered Objective-C methods use the signatures required by AppKit.
define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = DelegateState]
    struct Delegate;
    // SAFETY: NSObjectProtocol imposes no additional invariants.
    unsafe impl NSObjectProtocol for Delegate {}
    // SAFETY: Delegate is retained throughout NSApplication::run.
    unsafe impl NSApplicationDelegate for Delegate {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn launched(&self, _notification: &NSNotification) { self.ivars().controller.launch(); }
        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn reopen(&self, _app: &NSApplication, _visible: bool) -> bool {
            self.ivars().controller.request_activation(); false
        }
        #[unsafe(method(applicationWillTerminate:))]
        fn terminating(&self, _notification: &NSNotification) { self.ivars().controller.shutdown(); }
    }
    // SAFETY: NSMenuDelegate callbacks are delivered on AppKit's main thread.
    unsafe impl NSMenuDelegate for Delegate {
        #[unsafe(method(menuWillOpen:))]
        fn menu_will_open(&self, _menu: &NSMenu) {
            let controller = &self.ivars().controller;
            controller.menu_open.set(true); controller.stop();
        }
        #[unsafe(method(menuDidClose:))]
        fn menu_did_close(&self, _menu: &NSMenu) {
            let controller = &self.ivars().controller;
            controller.menu_open.set(false);
            if controller.activation_requested.replace(false) { controller.request_activation(); }
        }
    }
    impl Delegate {
        #[unsafe(method(activateOverlay:))]
        fn activate_overlay(&self, _sender: &NSObject) { self.ivars().controller.request_activation(); }
        #[unsafe(method(quit:))]
        fn quit(&self, _sender: &NSObject) {
            self.ivars().controller.shutdown(); NSApplication::sharedApplication(self.mtm()).terminate(None);
        }
        #[unsafe(method(displaysChanged:))]
        fn displays_changed(&self, _notification: &NSNotification) { self.ivars().controller.stop(); }
    }
);
struct DelegateState {
    controller: Rc<Controller>,
    status_item: OnceCell<Retained<NSStatusItem>>,
}
impl Delegate {
    fn new(mtm: MainThreadMarker, controller: Rc<Controller>) -> Retained<Self> {
        let allocated = Self::alloc(mtm).set_ivars(DelegateState {
            controller,
            status_item: OnceCell::new(),
        });
        // SAFETY: NSObject initializer has no arguments and initialized ivars.
        unsafe { msg_send![super(allocated), init] }
    }
    fn install_menu(&self) -> Result<()> {
        let mtm = self.mtm();
        let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
        let button = item
            .button(mtm)
            .context("could not create the menu bar button")?;
        button.setTitle(ns_string!("QC"));
        button.setToolTip(Some(ns_string!("QuadClicker")));
        let menu = NSMenu::new(mtm);
        menu.setAutoenablesItems(false);
        // SAFETY: Delegate is retained in run() until the menu is removed.
        menu.setDelegate(Some(ProtocolObject::from_ref(self)));
        for (title, selector) in [
            (ns_string!("Activate"), sel!(activateOverlay:)),
            (ns_string!("Quit"), sel!(quit:)),
        ] {
            // SAFETY: Selectors match the target's registered object-argument methods.
            let menu_item = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    NSMenuItem::alloc(mtm),
                    title,
                    Some(selector),
                    ns_string!(""),
                )
            };
            // SAFETY: Native menu uses a non-owning target; retained Delegate outlives it.
            unsafe {
                menu_item.setTarget(Some(self));
            }
            menu.addItem(&menu_item);
        }
        item.setMenu(Some(&menu));
        self.ivars()
            .status_item
            .set(item)
            .map_err(|_| anyhow::anyhow!("the status menu was installed twice"))?;
        // SAFETY: Selector accepts NSNotification; delegate stays alive until
        // observer removal after run() and notification center does not own it.
        unsafe {
            NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
                self,
                sel!(displaysChanged:),
                Some(ns_string!(
                    "NSApplicationDidChangeScreenParametersNotification"
                )),
                None,
            );
        }
        Ok(())
    }
}

pub fn run() -> Result<()> {
    let mtm = MainThreadMarker::new().context("QuadClicker must start on the main thread")?;
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    let controller = Controller::new(mtm);
    let delegate = Delegate::new(mtm, controller.clone());
    delegate.install_menu()?;
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    app.run();
    controller.shutdown();
    // SAFETY: Removing this live registered observer before delegate destruction.
    unsafe {
        NSNotificationCenter::defaultCenter().removeObserver(&delegate);
    }
    if let Some(item) = delegate.ivars().status_item.get() {
        NSStatusBar::systemStatusBar().removeStatusItem(item);
    }
    app.setDelegate(None);
    Ok(())
}
