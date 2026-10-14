# QuadClicker

A native macOS menu bar app that selects a point by repeatedly dividing the screen into four rectangles. Rust manages the navigation; AppKit draws the overlay and Core Graphics delivers clicks.

## Build and run

Requires macOS 13 or newer and a Rust toolchain. From this directory:

```sh
./scripts/build-app.sh
ditto dist/QuadClicker.app /Applications/QuadClicker.app
open /Applications/QuadClicker.app
```

The build produces a locally signed app at `dist/QuadClicker.app`; the next command installs it in `/Applications`. Run that installed copy for a consistent app identity. Quit QuadClicker before installing an updated build. Ad-hoc signing may require granting permission again after rebuilding.

## Accessibility permission

Open **System Settings → Privacy & Security → Accessibility** and enable **QuadClicker**. If it is absent, click **+**, press **Command+Shift+G**, enter `/Applications/QuadClicker.app`, and click **Open**. Authenticate if macOS asks. Turn on its switch, then choose **QC → Activate**, or reopen the app. If macOS requires a restart, choose **QC → Quit** and reopen it.

No Screen Recording permission is needed. After rebuilding, if macOS stops recognizing the permission, remove the app from the Accessibility list, add the newly built app, and reopen it.

## Navigation

Opening QuadClicker activates the overlay. Choose **QC → Activate** from the menu bar, or reopen the app, to start again at the whole display. Escape hides the overlay; **QC → Quit** exits the app.

Press the physical Mission Control button to open or close the overlay. The keyboard's normal brightness and volume behavior stays unchanged. Double Option and Fn+Space activation are deferred.

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

Navigation uses the physical U/I/J/K positions, independent of keyboard layout. Hold Shift if desired; Enter clicks only without modifiers. Other keys or unsupported shortcuts hide the overlay and pass through to the app underneath; Shift alone leaves it open. Holding a key does not repeat actions. Escape always hides the overlay.
