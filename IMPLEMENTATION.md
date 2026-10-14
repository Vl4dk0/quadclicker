# QuadClicker architecture

QuadClicker is a native macOS menu bar application. Rust owns navigation and input policy; public AppKit and Core Graphics APIs provide the overlay, keyboard interception, and mouse events. The native boundary uses `objc2` framework bindings. There is no virtual keyboard driver, private API, or system shortcut rewrite.

## State and modules

- `navigation`: finite positive rectangles, quadtree parent history, grid selection, and midpoint. Quadtree root depth is zero and allows nine subdivisions. Grid starts at 4×4, moves by whole cells without wrapping, and halves or doubles cells per side from 1 to 512. Both modes retain a one-point minimum cell dimension.
- `routing`: pure physical-key policy and press ownership. Active navigation commands are consumed; unrelated keys dismiss the overlay and pass through unchanged. A consumed press retains its repeats and key-up even after dismissal or modifier changes; only repeatable navigation commands execute on autorepeat.
- `macos/app`: application lifecycle, menu, activation, selected display, command execution, and native resource ownership. The menu contains Activate and Quit.
- `macos/input`: filtering session event tap, modifier translation, queued commands, and synthetic clicks.
- `macos/overlay`: retained nonactivating, non-key, mouse-transparent panel and drawing view. It redraws the selected rectangle, red dot with a contrast halo, and top-left mode label on state changes. Grid lines are shown when cells are at least 12 logical points wide and high; the exact selection and density label remain visible at smaller sizes. Subtle rounded selection corners scale down for tiny cells.
- `macos/display`: display selection and conversion between AppKit and Quartz coordinates.
- `macos/permissions`: Accessibility and event-posting checks, the initial native consent request, and recoverable error presentation.
- `macos/scheduling`: main-run-loop deferred work, without a polling timer or fixed sleep.

## Activation and input ownership

The filtering event tap stays available while idle, so the physical Mission Control button (virtual code 160) or Fn+Space can activate the app. Standard F3 (code 99) is distinct. Both activation shortcuts start a fresh Quadtree root when idle and close the overlay when active. Opening or reopening the application, or choosing Activate, also starts a fresh Quadtree session.

Fn+Space is recognized from the Space event’s SecondaryFn flag, rather than global modifier state, which can lag an event. Shift, Command, Option, and Control disqualify the chord. Caps Lock does not. The Mission Control event may also carry Function metadata.

A consumed press owns its down, repeats, and final up. Releasing Fn before Space therefore cannot leak the Space release. A Space press that began outside the shortcut remains owned by the target app; adding Fn during autorepeat cannot activate QuadClicker. Shift and Fn alone remain harmless prefixes while the overlay is active. Other modifier presses dismiss it. Escape always exits; keypad Enter retains native Enter behavior even when it carries Function metadata.

M switches modes around the current click target: the destination selection contains that point, although its midpoint can change. Grid density is retained within the active session. Navigation uses macOS autorepeat events and the user’s normal repeat settings; no repeat timer is added. Mode changes, clicks, resets, activation, and display switching execute once per press. Enter always clicks the selected rectangle's midpoint.

Quadtree uses U/I/J/K for quadrants. Grid uses H/J/K/L for left/down/up/right and U/I to halve/double cells per side. Resizing selects the cell containing the previous midpoint. Movement stops at display edges.

O and Backspace select the parent in Quadtree and do nothing in Grid. Shift+O and Shift+Backspace reset the current mode: Quadtree to its root, Grid to screen-centred 2×2. Unmodified H is Grid’s left command. Shift+H and Shift+L select the previous or next display in both modes.

Unrelated keys dismiss navigation immediately and pass the original event through. Dismissal clears pending commands, including clicks. A generation boundary prevents commands from an old activation from executing during a new one. Opening the app’s menu suspends navigation before AppKit handles menu keyboard input.

## Main thread and native lifetimes

AppKit objects and event-tap state stay on the AppKit main thread and its Core Foundation run loop. Tap callbacks only classify input and queue small state transitions. One coalesced `CFRunLoopPerformBlock` drain applies UI changes or posts clicks after releasing mutable borrows. Weak controller references prevent pending work from resurrecting a terminated app.

The callback context has a stable owned allocation. Teardown disables and invalidates the tap and removes its run-loop source before freeing that context. Native callbacks contain panic boundaries. Unsafe operations stay in small documented FFI blocks.

A disabled-tap notification ends that tap. Failure is latched, held-key ownership is cleared, and the tap is removed before one error is presented. A later activation can create a fresh tap. Modal permission/error dialogs block overlapping activation.

## Focus, clicks, and coordinates

The overlay never takes keyboard focus. Intercepted navigation remains available after a click changes the foreground application.

Clicking first allocates mouse-move, mouse-down, and mouse-up events, then posts them in that order at the same midpoint. Flags are explicitly unmodified and the button events use a single-click count. Allocation completes before any event is posted, preventing a partial click on allocation failure. Movement matters for native views that cache pointer coordinates from motion events, including Ghostty. There are no retries or timing sleeps; a target may intentionally consume the first click to activate its window or pane.

Displays are matched by ID between `NSScreen` and `CGDisplayBounds`. AppKit bottom-left coordinates are normalized within the selected screen and mapped to Quartz top-left bounds. Retina scaling is not applied to mouse coordinates. Display switching orders screens left to right, then top to bottom, and wraps. It preserves each mode’s normalized target position and grid density or quadtree depth while resizing and moving the overlay to the destination screen. Density or depth is reduced only when needed to keep cells at least one logical point. Explicit switching discards earlier queued clicks while preserving navigation and later commands in order; it does not start a new activation generation. Display reconfiguration closes the overlay so stale geometry cannot click an unintended location; the next activation selects a fresh display.

## Permissions and distribution

Accessibility consent is needed for the filtering tap and synthetic clicks. Startup makes one native consent request if necessary. Later Activate/reopen requests check permission and show an OK-only explanation when it is missing. The application does not automatically open Settings, poll consent, or request Screen Recording.

The bundle has a stable identifier (`sk.vladimirjancar.quadclicker`) and `LSUIElement` menu bar identity. Its icon is copied from `assets/QuadClicker.icns`; the menu bar uses a separate native symbol.

Public releases use free ad-hoc signing and are not notarized. Ad-hoc designated requirements depend on the binary hash, so updates can invalidate Accessibility grants. Local certificate-backed builds use an explicitly selected identity; a compatible default designated requirement can preserve consent across updates. There is no hardcoded personal signing identity, custom weakened identity rule, privacy-database modification, or automatic installation.

## Validation

The focused tests cover navigation, depth and minimum size, coordinate conversion, shortcut/modifier routing, repeat ownership, and dismissal. Run formatting, Clippy with warnings denied, tests, release compilation, and strict bundle signature verification.

Unit tests do not prove OS event delivery. Hardware-dependent checks include the Mission Control button, Fn+Space opening/closing without leaked spaces, real clicks, retained focus, Accessibility recovery, and multi-display/full-screen placement. Native click and Mission Control activation have been exercised on an Apple Silicon MacBook; every external keyboard and display configuration is not covered.

## API references

- [Apple: event filtering callback contract](https://developer.apple.com/documentation/coregraphics/cgeventtapcallback?language=objc).
- [Apple: mouse-transparent windows](https://developer.apple.com/documentation/appkit/nswindow/ignoresmouseevents?language=objc).
- [Apple: global display bounds](https://developer.apple.com/documentation/coregraphics/cgdisplaybounds%28_%3A%29?language=objc).
- [Apple keyboard source: virtual 0xA0 is Expose All](https://github.com/apple-oss-distributions/IOHIDFamily/blob/main/IOHIDFamily/IOHIDKeyboard.cpp#L729).
- [Apple: designated requirements and privacy grants](https://developer.apple.com/documentation/technotes/tn3127-inside-code-signing-requirements).
- [Apple: opening apps downloaded outside the App Store](https://support.apple.com/en-us/102445).
- [Apple: Accessibility permission setup](https://support.apple.com/guide/mac-help/mh43185/mac).
