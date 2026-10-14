# Installation and building

## Download and install

[Download the latest release](https://github.com/Vl4dk0/quadclicker/releases/latest). Releases support **Apple Silicon (arm64), macOS 13 or newer**. Intel builds are not provided.

1. Download and unzip the app archive, then move **QuadClicker.app** into **Applications**.
2. Open it. This free release is ad-hoc signed and is **not notarized by Apple**. If macOS blocks it as an unidentified developer, follow [Apple’s opening instructions](https://support.apple.com/en-us/102445): after trying to open it, go to **System Settings → Privacy & Security → Open Anyway**, then confirm **Open** if you trust the download.
3. Go to **System Settings → Privacy & Security → Accessibility** and enable **QuadClicker**. If it is missing, click **+**, press **Command+Shift+G**, enter `/Applications/QuadClicker.app`, and click **Open**. Authenticate if asked.
4. Choose **Activate** from QuadClicker’s menu bar menu, or quit and reopen the app after granting permission.

Accessibility permission lets QuadClicker intercept its controls and send mouse clicks. No Screen Recording permission is needed. [Apple’s Accessibility guide](https://support.apple.com/guide/mac-help/mh43185/mac) explains the permission.

## Keyboard and overlay details

The Mission Control icon button is often in the F3 position; standard F3 is a different event. Activation has been tested on an Apple Silicon MacBook keyboard. Brightness and volume keys retain their normal behavior.

The overlay starts on the display containing the pointer. It stays out of keyboard focus and lets physical mouse clicks pass through. Navigation uses physical key positions, independent of keyboard layout. 

The last mode is remembered across activations and app restarts. Each activation resets navigation to a Quadtree root or a 2×2 Grid; previous selection, density, and display are not restored. Grid also starts at 2×2 when first selected in a session.

Holding navigation keys repeats using macOS’s keyboard repeat delay and speed: H/J/K/L and U/I in Grid; U/I/J/K, O, and Backspace in Quadtree. Click, mode switch, reset, activation, and display switch act once per press. 

Grid movement stops at display edges. Grid density ranges from 1×1 to 512×512, limited by a minimum one-point cell size. 

Shift+H and Shift+L switch to the previous or next display in both modes. Displays are ordered left to right, then top to bottom, with wraparound. Switching preserves the mode, grid density or quadtree depth, and the target’s relative position; depth or density is reduced only when needed to keep cells at least one logical point. Other supported Shift direction combinations retain their navigation behavior. 

Enter clicks without modifiers; native keypad Enter is also supported. New unsupported key presses silently dismiss the overlay; their down, repeats, and release are consumed. Modifier changes still pass through, and keys already held before activation keep their original ownership.

## Updates and limitations

Quit QuadClicker before replacing its app bundle. Because free ad-hoc signing identifies each build separately, an update may require granting Accessibility permission again. If macOS stops recognizing the grant, remove only the stale **QuadClicker** entry, add `/Applications/QuadClicker.app` again, and reopen it.

An application may use the first click to activate its window or pane. Multi-display and full-screen behavior has not been tested on every configuration. Activation depends on the keyboard events macOS delivers; external keyboards may differ.

## Build from source

Requires macOS, Rust 1.85 or newer, and Apple’s Command Line Tools (`xcode-select --install` if missing).

```sh
git clone https://github.com/Vl4dk0/quadclicker.git
cd quadclicker
QUADCLICKER_SIGNING_IDENTITY=- ./scripts/build-app.sh
```

The result is `dist/QuadClicker.app`. Quit any installed copy, move the built app into Applications, and follow the permission instructions above. Ad-hoc signing (`-`) needs no Apple Developer account.

For your own certificate-backed local builds, set `QUADCLICKER_SIGNING_IDENTITY` to an existing code-signing identity instead. Keeping the same certificate and compatible designated requirement across updates can preserve Accessibility consent; switching signing identities may require consent again. The build script never chooses a private certificate or installs the app automatically.

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

To create the Apple Silicon release archive and SHA-256 checksum without installing it:

```sh
./scripts/package-release.sh
```

Artifacts are written to `target/releases/v0.3.1/`.

See [IMPLEMENTATION.md](IMPLEMENTATION.md) for the module boundaries and native API design, and [assets/README.md](assets/README.md) for icon provenance.
