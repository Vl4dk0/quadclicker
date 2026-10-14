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

When permission is missing, QuadClicker requests Accessibility access through macOS and opens **System Settings → Privacy & Security → Accessibility**. You can also open setup through **QC → Accessibility Setup…** in the menu bar. Enable **QuadClicker**. If it is absent, click **+**, press **Command+Shift+G**, enter `/Applications`, select **QuadClicker.app**, and click **Open**. Authenticate if macOS asks. Turn on its switch, then choose **QC → Activate**, or reopen the app. If macOS requires a restart, quit QuadClicker from its menu and reopen it.

No Screen Recording permission is needed. After rebuilding, if macOS stops recognizing the permission, remove the app from the Accessibility list, add the newly built app, and reopen it.

## Navigation

Opening QuadClicker activates the overlay. Choose **QC → Activate** from the menu bar, or reopen the app, to start again at the whole display. Escape hides the overlay; **Quit QuadClicker** exits the app.

Physical Mission Control button activation is implemented and still needs testing on the Mac's hardware; use the menu or reopen the app if that button does not activate it. Keep the keyboard's normal brightness and volume behavior. Double Option and Fn+Space activation are deferred.

Each direction selects one quarter of the current rectangle:

```text
U: top left       I: top right
J: bottom left    K: bottom right
```

| Key | Action |
| --- | --- |
| Backspace | Return to the previous rectangle |
| Shift+Backspace | Return to the whole display |
| Enter | Left-click at the centre dot; keep the current rectangle |
| Escape | Hide the overlay |

The starting rectangle is the display containing the pointer. The overlay lets mouse clicks pass through to the app beneath it.

Navigation uses the physical U/I/J/K positions, independent of keyboard layout. Hold Shift if desired; Enter clicks only without modifiers. Other keys or unsupported shortcuts hide the overlay and pass through to the app underneath; modifier keys alone leave it open. Holding a key does not repeat actions. Escape always hides the overlay.
