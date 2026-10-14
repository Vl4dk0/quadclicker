# QuadClicker

Click anywhere using your keyboard.

[Download](https://github.com/Vl4dk0/quadclicker/releases/latest) · [Install and build](INSTALLATION.md)

| Key | Both modes |
| --- | --- |
| Fn+Space or Mission Control button | Open or close overlay |
| M | Switch mode |
| Shift+H / Shift+L | Previous / next display |
| O or Backspace | Go back one level in Quadtree; no action in Grid |
| Shift+O or Shift+Backspace | Reset current mode |
| Enter | Click the centre dot; keep selection |
| Escape or other key | Hide overlay silently; consume the key press |

| Mode | Controls |
| --- | --- |
| Quadtree | U / I / J / K: top left / top right / bottom left / bottom right quarter; up to nine subdivisions |
| Grid | H / J / K / L: left / down / up / right. U halves cells per side; I doubles them |

Hold navigation keys to repeat using macOS’s keyboard repeat delay and speed. Click, mode switch, reset, activation, and display switch run once per press.

The last mode is remembered, including after restarting the app. Each activation starts fresh: Quadtree at its root or Grid at **2×2**, without restoring the previous selection. Reset also returns Grid to **2×2**. Switching modes preserves the target area. Display switching wraps from left to right, then top to bottom, preserving mode, grid density or quadtree depth, and the target’s relative position. Depth or density is reduced only if cells would be smaller than one logical point on the destination display. The top-left label shows the mode; the red dot marks the click.

Choose **Activate** or **Quit** from the menu. Controls use physical key positions. Shift or Fn alone keeps the overlay open. New unrelated key presses close the overlay without reaching the app underneath.
