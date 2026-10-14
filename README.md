# QuadClicker

A native macOS menu bar app that selects a point by repeatedly dividing the screen into four rectangles. Rust manages the navigation; AppKit draws the overlay and Core Graphics delivers clicks.

## Build and run

Requires macOS 13 or newer, a Rust toolchain, and this Mac's configured Apple Development signing identity. From this directory:

```sh
./scripts/build-app.sh
ditto dist/QuadClicker.app /Applications/QuadClicker.app
open /Applications/QuadClicker.app
```

The build produces a signed app at `dist/QuadClicker.app`; the next command installs it in `/Applications`. Run that installed copy for a consistent app identity. Quit QuadClicker before installing an updated build. The script uses this Mac's existing Apple Development certificate and fails if it is unavailable; it does not fall back to ad-hoc signing.

## Accessibility permission

On first launch, macOS may prompt for Accessibility access. Open **System Settings → Privacy & Security → Accessibility** and enable **QuadClicker**. If it is absent, click **+**, press **Command+Shift+G**, enter `/Applications/QuadClicker.app`, and click **Open**. Authenticate if macOS asks. Turn on its switch, then choose **Activate** from QuadClicker's menu bar menu, or reopen the app. If macOS requires a restart, choose **Quit** from its menu and reopen it.

No Screen Recording permission is needed. Switching from an earlier ad-hoc build to the certificate-signed app requires granting permission again once. If macOS no longer recognizes the permission, remove the stale QuadClicker entry, add `/Applications/QuadClicker.app`, and reopen it.

## Navigation

Opening QuadClicker activates the overlay. Choose **Activate** from its menu bar menu, or reopen the app, to start again at the whole display. Escape hides the overlay; **Quit** exits the app.

Press the physical Mission Control button or **Fn+Space** to open or close the overlay. The keyboard's normal brightness and volume behavior stays unchanged. Double Option activation is deferred.

Each direction selects one quarter of the current rectangle:

```text
U: top left       I: top right
J: bottom left    K: bottom right
```

Navigation allows at most nine subdivisions; further quadrant presses leave the rectangle unchanged until you go back or reset.

| Key | Action |
| --- | --- |
| Backspace | Return to the previous rectangle |
| Shift+Backspace | Return to the whole display |
| Enter | Left-click at the centre dot; keep the current rectangle |
| Escape | Hide the overlay |

The starting rectangle is the display containing the pointer. The overlay lets mouse clicks pass through to the app beneath it.

Navigation uses the physical U/I/J/K positions, independent of keyboard layout. Hold Shift if desired; Enter clicks only without modifiers. Other keys or unsupported shortcuts hide the overlay and pass through to the app underneath; Shift or Fn alone leaves it open. Holding a key does not repeat actions. Escape always hides the overlay.
