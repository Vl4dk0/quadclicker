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
| Grid | H / J / K / L: left / down / up / right; Ctrl moves three cells. Shift+J doubles cells per side; Shift+K halves them |

Hold navigation keys to repeat using macOS’s keyboard repeat delay and speed. In Grid, hold two directions together for diagonal movement, such as H+J for down-left. Opposite directions cancel on their axis; releasing one key keeps the other moving. Ctrl changes the stride to three cells, including while keys are held. Click, mode switch, hint toggle, reset, activation, and display switch run once per press.

New users start in a **32×32 Grid**. The last mode, Grid density, selected cell, display, and hint visibility are remembered, including after restarting the app. Hints are hidden by default; when enabled, small H/J/K/L labels show the next click targets. Grid reopens where you left its red dot; opening the overlay does not move the mouse pointer. If its display is unavailable, Grid restores the relative position on the display containing the pointer. Quadtree still starts at its root on the pointer’s display. Reset returns Grid to centred **2×2** and remembers that selection. Switching modes preserves the target area. Display switching wraps from left to right, then top to bottom, preserving mode, grid density or quadtree depth, and the target’s relative position. Depth or density is reduced only if cells would be smaller than one logical point on the destination display. The top-left label shows the mode; the red dot marks the click. Quadtree shows thin 2×2 dividing lines inside the selected rectangle while another subdivision is available.

Choose **Activate** or **Quit** from the menu. Controls use physical key positions. Shift or Fn alone keeps the overlay open; Control does too in Grid. New unrelated key presses close the overlay without reaching the app underneath.
