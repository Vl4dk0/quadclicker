# QuadClicker implementation plan

Build a small native macOS menu bar app in Rust. Use public AppKit and Core Graphics APIs through `objc2`; keep the quadtree state independent of macOS. The first release is a local Apple Silicon app activated by the physical Mission Control icon button alone. Preserve the default direct brightness, volume, and other function-row behavior. Shortcut customization and alternate activation gestures come later.

## User flow

Opening `QuadClicker.app` requests activation. When permission is available, show a transparent overlay on the display containing the pointer, using that display's full bounds as the root. Opening the already-running app activates a fresh root again. Escape hides the overlay and leaves the menu bar app running. Its menu contains Activate, Accessibility Setup, and Quit; Activate runs after the menu has closed, without a fixed sleep or stealing the target application's focus.

Use a public filtering `CGEventTap` to handle the Mission Control button's actual keyboard event and consume its down/repeat/up sequence. Apple open-source keyboard tables identify virtual keycode `0xA0` (160) as Expose All, distinct from standard F3 (`0x63`, 99). This is the implementation candidate; a physical press on the user's Mac must verify both delivery and suppression before claiming acceptance. Select a public tap position that receives the event before Mission Control handles it; prefer the existing session head tap if verified, otherwise evaluate the public HID tap with actual permissions and creation results. Do not use private APIs, a virtual keyboard driver, preference rewrites, or synthetic-event success as a substitute for the physical test.

Keep the tap enabled while the app is idle, but route only the unmodified Mission Control button to activation; pass all unrelated input unchanged. Navigation commands are enabled only while the overlay is open. Each distinct Mission Control press activates a fresh root through the same application action as the menu. Holding it must not repeatedly reset the selection. Preserve menu/reopen activation if the tap is unavailable. Do not require Fn, switch the entire function row to standard keys, or change system shortcut settings automatically. Double Option and Fn+Space are out of scope.

The user separately wants the physical Spotlight icon button to open Raycast. Keep that setup independent: do not add a Raycast-launch action or consume the Spotlight button in QuadClicker. Raycast's documented native hotkey settings should be evaluated first; physical-icon support is not established merely by its documented Command+Space replacement. Any fallback single-key remapping needs its own verified mapping and user-approved setup.

The overlay shows only the selected rectangle and its midpoint dot. Use a contrasting outline that remains visible on light and dark content. Redraw on state changes rather than running an animation loop. Put controls in the README and menu help, keeping the overlay uncluttered.

| Key | Action |
| --- | --- |
| U / I | Select top-left / top-right quarter |
| J / K | Select bottom-left / bottom-right quarter |
| Backspace | Return to parent; root is a no-op |
| Shift+Backspace | Return to root |
| Return or keypad Enter | Single left click at midpoint; keep selection and overlay |
| Escape | Hide overlay |
| Other key or unsupported shortcut | Hide overlay and pass the original event to the underlying app |

Use physical U/I/J/K key positions for a predictable navigation cluster; document this. Ignore Caps Lock for commands. Allow Shift on quadrant keys; only unmodified Enter clicks. Command/Control/Option combinations dismiss the overlay and pass through, except Escape always exits and is consumed. Unrecognized key-down events also dismiss and pass through. Pressing a modifier alone keeps the overlay open. Ignore autorepeat for owned navigation commands: one physical press performs one action. A repeating key whose press began outside the overlay still belongs to the underlying app; its repeat dismisses and passes through.

## Modules and ownership

- `navigation`: validated finite positive rectangles, quadrant enum, root/parent history, midpoint, and subdivision limit. Store rectangles in logical points. Stop subdivision when either child dimension would be smaller than one point; this naturally bounds history without a configurable depth flag.
- `app`: native application/delegate, status menu, activation lifecycle, permission/error presentation, selected display, and ownership of native resources.
- `overlay`: retained nonactivating borderless `NSPanel` and drawing view. Never key/main; clear background, non-opaque, no shadow, mouse-transparent, and visible while the app is inactive. Use public window levels and collection behavior for the active Space/full-screen companion behavior.
- `input`: public filtering event tap, idle Mission Control activation, active-session navigation, paired release tracking, plus synthetic mouse event creation/posting. Keep pointer and callback invariants documented at the native boundary. The physical-button activation belongs in the same small typed input policy; a Carbon F3 registration module is unnecessary.
- A small display/coordinate helper may become its own module if needed; avoid a general plugin or backend abstraction.

Use a single AppKit main thread and its Core Foundation run loop. AppKit objects and event-tap state stay on that thread; no Tokio runtime or worker thread is needed. Register the tap source in common run-loop modes. Retain the application delegate, menu/status item, panel/view, tap and run-loop source for their complete lifetimes.

The callback context has a stable owned allocation. Its lifetime ends only after the tap is disabled/invalidated and its run-loop source removed. Keep callback work to input classification and a short state transition; release mutable borrows before any AppKit call. Queue actions in order and schedule one pending drain with `CFRunLoopPerformBlock` plus `CFRunLoopWakeUp`, using common modes. The next-turn drain releases queue/policy borrows before applying AppKit updates or posting clicks. Retain only weak controller references so pending work cannot resurrect a terminated app. This avoids a perpetual polling timer and its idle work/fixed latency. Do not display alerts or perform blocking work inside the event callback. Never let a Rust panic unwind across the native callback boundary.

Keep a set of keys whose down event was consumed. Consume their repeats and final up event even if Escape or unrelated typing has already hidden the overlay or modifiers changed. Deactivate navigation handling immediately on Escape or an unrelated key-down; continue draining outstanding consumed releases while the tap remains available for Mission Control activation. An unrelated key never acquires ownership: forward its original event without synthesizing a replacement. Both dismissal paths clear queued actions, including pending clicks; a fresh activation cannot execute stale queued clicks. Opening the app's own status menu suspends navigation before menu keyboard handling.

## Clicks, focus, and display coordinates

The panel remains non-key and click-through. A filtering `CGEventTap` consumes commands before they reach the target application. This preserves navigation after a click changes the foreground window, without repeatedly taking keyboard focus back.

Create both mouse down and up events before posting either; handle allocation failure before emitting a partial click. Post a matched pair at the current midpoint with a single-click count and explicit unmodified flags. Clicks do not change navigation state. Do not add retries, double clicks, or arbitrary timing sleeps to compensate for a target application's own first-click behavior.

Match `NSScreen` and `CGDisplayBounds` by display ID. Keep AppKit screen geometry and Quartz event coordinates distinct and convert at the boundary: for a selected screen's bottom-left AppKit point, derive its normalized local position and map into that display's top-left Quartz bounds. This handles negative origins and avoids multiplying by Retina scale. Use AppKit backing conversion only for crisp drawing. Close the active overlay when displays are reconfigured, so stale coordinates cannot click an unintended location; the next activation selects a fresh root.

## Permission and failure UX

Accessibility consent is required for the active filtering tap and synthetic clicks. A listen-only global keyboard monitor cannot suppress navigation keys. Do not request Screen Recording or automatically request Input Monitoring for this architecture.

Check Accessibility trust and event-posting access on activation, then check actual tap creation. If Accessibility trust is absent, keep navigation inactive and call `AXIsProcessTrustedWithOptions` with `kAXTrustedCheckOptionPrompt` set to `kCFBooleanTrue`, so macOS receives a permission request from the installed application. Its prompt is asynchronous; the immediate return value does not represent new consent. If Accessibility is trusted but event posting is unavailable, call `CGRequestPostEventAccess` instead. Open the Accessibility privacy page without stacking an app-owned setup alert over the native prompt. Recheck on the next explicit Activate/reopen; do not poll or repeatedly prompt. A revoked grant or tap failure hides the overlay and provides a recoverable error via the app UI. A disabled-tap notification is terminal for that tap: latch failure once, remove and invalidate the tap before presenting one error, and create a fresh tap on a later activation.

Block activation while an app-owned modal setup/error dialog is presented, including activation arriving through the idle Mission Control tap. Restore that guard when the dialog exits, and preserve paired releases already consumed. When macOS disables the tap, end navigation and clear stale held-key ownership because releases may have been missed; normal Escape must continue preserving paired releases.

Exact README setup instructions:

1. Build with `./scripts/build-app.sh`, quit any running QuadClicker, install with `ditto dist/QuadClicker.app /Applications/QuadClicker.app`, and launch with `open /Applications/QuadClicker.app`.
2. The app requests permission through macOS and opens **System Settings > Privacy & Security > Accessibility**.
3. Enable **QuadClicker**. If absent, press **+**, then **Command+Shift+G**, enter `/Applications`, select `QuadClicker.app`, and click **Open**. Authenticate if macOS asks.
4. Choose **QuadClicker > Activate** from the menu bar, or reopen the app. If macOS requires a restart, quit QuadClicker from its menu and reopen it.

This is the Accessibility permission page under Privacy & Security, distinct from the general Accessibility settings section. If a rebuilt app is no longer trusted, remove the stale QuadClicker entry and add this exact bundle again; do not instruct the user to reset all privacy permissions.

## Build and code quality

Provide one readable `scripts/build-app.sh` that builds release mode, assembles `dist/QuadClicker.app/Contents`, writes/copies its Info.plist, and ad-hoc signs and verifies the bundle. Use a constant bundle ID (`sk.vladimirjancar.quadclicker`), executable name, and `LSUIElement` menu bar identity. Quote paths and stop on failure. Commit Cargo.lock; ignore build outputs. After quitting the running app, install the built bundle with `ditto dist/QuadClicker.app /Applications/QuadClicker.app`; launch only the installed copy with `open /Applications/QuadClicker.app` so consent belongs to the intended application and stable installation path. Stable path/ID help identity, but ad-hoc signing does not guarantee permission persistence across changed binaries; do not claim otherwise.

Use Rust 2024, an explicit supported Rust version, selected framework crate features, and straightforward `Result` propagation. Configure shared Rust/Clippy lints, including no production unwrap/expect. Deny unsafe by default and allow it only at documented macOS FFI operations; the user's preference against pointless flags takes precedence over adding a Cargo feature solely to permit required FFI. Keep unsafe blocks small with ownership/thread/lifetime safety comments. Use purpose-oriented comments for non-obvious design choices.

## Acceptance

Retain a small suite of roughly 6–10 meaningful tests covering navigation paths/undo/reset, bounds and minimum size, midpoint/coordinate conversion across negative and mixed-scale display layouts, modifier/repeat routing, paired releases across Escape, and click preserving selection. Do not add a test per trivial accessor.

Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, release build, and bundle signature verification. Inspect the actual desktop overlay and verify real clicks against a harmless target where permission allows: launch/reopen/Activate, all quadrants, parent/root, repeated Enter, Escape then normal typing, foreground focus, menu interaction, Retina placement, permission denied, and permission recovery. Check full-screen Spaces and additional displays when available. Report unavailable hardware/permission-dependent checks explicitly; compilation and unit tests do not prove OS event delivery. Miri cannot validate AppKit/Core Graphics calls; any Miri check is limited to extracted pure logic rather than presented as native FFI proof.

With the user's existing keyboard settings intact, physically press the Mission Control icon button and verify that QuadClicker opens while Mission Control does not. Verify brightness and volume still respond directly, Fn behavior remains unchanged, and the Spotlight button passes through. Check repeat/release behavior and activation again after Escape. A temporary read-only diagnostic may establish the actual event path when necessary; do not retain a general key logger or log typed content. Synthetic keycode-160 tests only verify routing, not hardware delivery or suppression.

## API evidence

- [Rust AppKit bindings](https://docs.rs/objc2-app-kit/latest/objc2_app_kit/struct.NSPanel.html) and [Core Graphics event bindings](https://docs.rs/objc2-core-graphics/latest/objc2_core_graphics/struct.CGEvent.html).
- [Apple: mouse-transparent windows](https://developer.apple.com/documentation/appkit/nswindow/ignoresmouseevents?language=objc) and [event filtering callback contract](https://developer.apple.com/documentation/coregraphics/cgeventtapcallback?language=objc).
- [Apple: filtering versus listening permissions](https://developer.apple.com/videos/play/wwdc2019/701/) and [Accessibility permission setup](https://support.apple.com/guide/mac-help/mh43185/mac).
- [Apple: global display bounds](https://developer.apple.com/documentation/coregraphics/cgdisplaybounds%28_%3A%29?language=objc) and [point/backing coordinate guidance](https://developer.apple.com/library/archive/documentation/GraphicsAnimation/Conceptual/HighResolutionOSX/APIs/APIs.html).
- [Apple keyboard source: virtual 0xA0 is Expose All](https://github.com/apple-oss-distributions/IOHIDFamily/blob/main/IOHIDFamily/IOHIDKeyboard.cpp#L729). This is source evidence, not proof of the current physical key's delivery path.
- [Raycast native hotkey settings](https://manual.raycast.com/settings) document Command+Space replacement; they do not establish physical Spotlight icon support on this Mac.
