use objc2::MainThreadMarker;
use objc2_app_kit::NSAlert;
use objc2_core_foundation::{CFBoolean, CFDictionary, CFString};
use objc2_core_graphics::{CGPreflightPostEventAccess, CGRequestPostEventAccess};
use objc2_foundation::{NSString, ns_string};

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> u8;
    fn AXIsProcessTrustedWithOptions(options: &CFDictionary) -> u8;
    static kAXTrustedCheckOptionPrompt: &'static CFString;
}

pub fn granted() -> bool {
    // SAFETY: AXIsProcessTrusted takes no pointers and only checks this process.
    (unsafe { AXIsProcessTrusted() } != 0) && CGPreflightPostEventAccess()
}

/// Request consent through macOS itself so TCC records this exact installed
/// application. Opening Settings alone never registers a permission request.
pub fn request_on_launch() {
    // SAFETY: Both the trust API and its immutable option key are public symbols
    // from ApplicationServices. The typed dictionary retains its CFString key
    // and CFBoolean value throughout the synchronous native call.
    let trusted = unsafe { AXIsProcessTrusted() } != 0;
    let posting_allowed = CGPreflightPostEventAccess();
    eprintln!(
        "QuadClicker: permission setup (Accessibility={trusted}, event posting={posting_allowed})"
    );
    if !trusted {
        // SAFETY: The framework owns this non-null, immutable CFString constant.
        let key = unsafe { kAXTrustedCheckOptionPrompt };
        let options = CFDictionary::from_slices(&[key], &[CFBoolean::new(true)]);
        // SAFETY: The dictionary contains the documented Boolean prompt option.
        // The system prompt is asynchronous; its return is not treated as consent.
        unsafe { AXIsProcessTrustedWithOptions(options.as_ref()) };
    } else if !posting_allowed {
        // Accessibility and event-posting checks can disagree after a permission
        // change. Ask the posting API itself rather than pretending trust grants it.
        CGRequestPostEventAccess();
    }
    // The system prompt is asynchronous. Activation waits for an explicit
    // reopen or menu action after consent; there is no permission polling.
}

pub fn explain_missing_permission(mtm: MainThreadMarker) {
    let alert = NSAlert::new(mtm);
    alert.setMessageText(ns_string!("QuadClicker needs Accessibility permission"));
    alert.setInformativeText(ns_string!("Open System Settings → Privacy & Security → Accessibility and enable QuadClicker. If it is missing, use + to add /Applications/QuadClicker.app. Then choose Activate again. If macOS requires a restart, choose Quit and reopen the app."));
    alert.addButtonWithTitle(ns_string!("OK"));
    alert.runModal();
}

pub fn error(mtm: MainThreadMarker, message: &str) {
    let alert = NSAlert::new(mtm);
    alert.setMessageText(ns_string!("QuadClicker could not continue"));
    alert.setInformativeText(&NSString::from_str(message));
    alert.addButtonWithTitle(ns_string!("OK"));
    alert.runModal();
}
