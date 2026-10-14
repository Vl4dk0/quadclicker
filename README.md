# QuadClicker

Click anywhere using your keyboard.

[Download](https://github.com/Vl4dk0/quadclicker/releases/latest) · [Install and build](INSTALLATION.md)

| Key | Both modes |
| --- | --- |
| Fn+Space or Mission Control button | Open or close overlay |
| M | Switch mode |
| T | Show or hide destination hints |
| Shift+H / Shift+L | Previous / next display |
| O or Backspace | Go back one level in Quadtree; no action in Grid |
| Shift+O or Shift+Backspace | Reset current mode |
| Enter | Click the centre dot; keep selection |
| Escape or other key | Hide overlay silently; consume the key press |

| Mode | Controls |
| --- | --- |
| Quadtree | H / J / K / L: top left / bottom left / bottom right / top right quarter; up to nine subdivisions |
| Grid | H / J / K / L: left / down / up / right. Shift+J doubles cells per side; Shift+K halves them |

Hold navigation keys to repeat using macOS’s keyboard repeat delay and speed. Click, mode switch, hint toggle, reset, activation, and display switch run once per press.

New users start in Grid. The last mode and hint visibility are remembered, including after restarting the app. Hints are hidden by default; when enabled, small H/J/K/L labels show the next click targets. Each activation starts fresh: Quadtree at its root or Grid at **2×2**, without restoring the previous selection. Reset also returns Grid to **2×2**. Switching modes preserves the target area. Display switching wraps from left to right, then top to bottom, preserving mode, grid density or quadtree depth, and the target’s relative position. Depth or density is reduced only if cells would be smaller than one logical point on the destination display. The top-left label shows the mode; the red dot marks the click.

Choose **Activate** or **Quit** from the menu. Controls use physical key positions. Shift or Fn alone keeps the overlay open. New unrelated key presses close the overlay without reaching the app underneath.
