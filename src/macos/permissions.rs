use objc2::MainThreadMarker;
use objc2_app_kit::{NSAlert, NSAlertFirstButtonReturn, NSWorkspace};
use objc2_core_graphics::CGPreflightPostEventAccess;
use objc2_foundation::{NSString, NSURL, ns_string};

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

pub fn granted() -> bool {
    // SAFETY: AXIsProcessTrusted takes no pointers and only checks this process.
    (unsafe { AXIsProcessTrusted() }) && CGPreflightPostEventAccess()
}

pub fn open_settings() {
    if let Some(url) = NSURL::URLWithString(ns_string!(
        "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
    )) {
        NSWorkspace::sharedWorkspace().openURL(&url);
    }
}

pub fn setup(mtm: MainThreadMarker) {
    let alert = NSAlert::new(mtm);
    alert.setMessageText(ns_string!("Allow QuadClicker in Accessibility"));
    alert.setInformativeText(ns_string!("QuadClicker needs permission to intercept its navigation keys and click. In System Settings → Privacy & Security → Accessibility, add this QuadClicker.app with + and enable it. Then choose Activate from its menu again."));
    alert.addButtonWithTitle(ns_string!("Open System Settings"));
    alert.addButtonWithTitle(ns_string!("Cancel"));
    if alert.runModal() == NSAlertFirstButtonReturn {
        open_settings();
    }
}

pub fn error(mtm: MainThreadMarker, message: &str) {
    let alert = NSAlert::new(mtm);
    alert.setMessageText(ns_string!("QuadClicker could not continue"));
    alert.setInformativeText(&NSString::from_str(message));
    alert.addButtonWithTitle(ns_string!("OK"));
    alert.runModal();
}
