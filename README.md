# QuadClicker

A native macOS menu bar app that selects a point by repeatedly dividing the screen into four rectangles. Rust manages the navigation; AppKit draws the overlay and Core Graphics delivers clicks.

## Build and run

Requires macOS 13 or newer and a Rust toolchain. From this directory:

```sh
./scripts/build-app.sh
open dist/QuadClicker.app
```

The build produces a locally signed app at `dist/QuadClicker.app`. Keep it at that location for a consistent app identity. Ad-hoc signing may require granting permission again after rebuilding.

## Accessibility permission

The first launch offers to open **System Settings → Privacy & Security → Accessibility**. You can also open it through **QC → Accessibility Setup…** in the menu bar. Click **+**, press **Command+Shift+G**, enter `/Users/vladko/Projects/quadclicker/dist`, and select **QuadClicker.app**. Turn on its switch, then choose **QC → Activate**, or reopen the app. If macOS requires a restart, quit QuadClicker from its menu and reopen it.

No Screen Recording permission is needed. After rebuilding, if macOS stops recognizing the permission, remove the app from the Accessibility list, add the newly built app, and reopen it.

## Navigation

Opening QuadClicker activates the overlay. Press the standard **F3** key, choose **QC → Activate** from the menu bar, or reopen the app, to start again at the whole display. Escape hides the overlay; **Quit QuadClicker** exits the app.

On an Apple keyboard with the default function-key behavior, press **Fn+F3**. For F3 alone, open **System Settings → Keyboard → Keyboard Shortcuts → Function Keys** and enable **Use F1, F2, etc. keys as standard function keys**. This changes the whole function-key row. In **Keyboard Shortcuts → Mission Control**, disable or reassign any shortcut using F3 if it conflicts. See [Apple's function-key instructions](https://support.apple.com/en-us/102439). Double Option and Fn+Space activation are deferred.

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

Navigation uses the physical U/I/J/K positions, independent of keyboard layout. Hold Shift if desired; Enter clicks only without modifiers. Command/Control/Option combinations pass through, and holding a key does not repeat actions. Escape always hides the overlay.
