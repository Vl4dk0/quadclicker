use crate::navigation::{Point, Rect, event_point};
use anyhow::{Context, Result};
use objc2::MainThreadMarker;
use objc2_app_kit::NSScreen;
use objc2_core_graphics::{CGDisplayBounds, CGEvent};
use objc2_foundation::{NSNumber, NSRect, ns_string};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Display {
    pub id: u32,
    pub appkit: NSRect,
    pub quartz: Rect,
}

impl Display {
    pub fn under_pointer(mtm: MainThreadMarker) -> Result<Self> {
        let event = CGEvent::new(None).context("could not read the pointer position")?;
        let point = CGEvent::location(Some(&event));
        Self::all(mtm)?
            .into_iter()
            .find(|display| {
                display.quartz.contains(Point {
                    x: point.x,
                    y: point.y,
                })
            })
            .context("the pointer is not on an available display")
    }
    fn all(mtm: MainThreadMarker) -> Result<Vec<Self>> {
        NSScreen::screens(mtm)
            .iter()
            .map(|screen| {
                let description = screen.deviceDescription();
                let id = description
                    .objectForKey(ns_string!("NSScreenNumber"))
                    .and_then(|number| number.downcast::<NSNumber>().ok())
                    .context("a display has no numeric screen identifier")?
                    .unsignedIntValue();
                let bounds = CGDisplayBounds(id);
                let quartz = Rect::new(
                    bounds.origin.x,
                    bounds.origin.y,
                    bounds.size.width,
                    bounds.size.height,
                )
                .context("a display has invalid bounds")?;
                Ok(Self {
                    id,
                    appkit: screen.frame(),
                    quartz,
                })
            })
            .collect()
    }
    /// A stable physical order also gives vertically stacked screens a cycle.
    /// Read the live screen list on each press, rather than caching hotplug state.
    pub fn adjacent(self, mtm: MainThreadMarker, next: bool) -> Result<Self> {
        let mut displays = Self::all(mtm)?;
        sort_displays(&mut displays);
        adjacent_display(&displays, self.id, next).context("no display is currently available")
    }
    pub fn unchanged(self, mtm: MainThreadMarker) -> bool {
        Self::all(mtm).is_ok_and(|displays| displays.contains(&self))
    }
    pub fn local_bounds(self) -> Result<Rect> {
        Rect::new(0.0, 0.0, self.appkit.size.width, self.appkit.size.height)
            .context("the selected display has invalid logical dimensions")
    }
    pub fn click_point(self, local: Point) -> Point {
        event_point(
            local,
            (self.appkit.size.width, self.appkit.size.height),
            self.quartz,
        )
    }
}

fn sort_displays(displays: &mut [Display]) {
    displays.sort_by(|left, right| {
        left.quartz
            .x
            .total_cmp(&right.quartz.x)
            .then_with(|| left.quartz.y.total_cmp(&right.quartz.y))
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn adjacent_display(displays: &[Display], current_id: u32, next: bool) -> Option<Display> {
    let index = displays.iter().position(|display| display.id == current_id);
    let target = match index {
        Some(index) if next => (index + 1) % displays.len(),
        Some(index) => {
            if index == 0 {
                displays.len() - 1
            } else {
                index - 1
            }
        }
        // The active display disappeared before AppKit's notification arrived.
        None => 0,
    };
    displays.get(target).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_cycle_wraps_handles_one_screen_and_missing_active_screen() {
        let display = |id, x, y| Display {
            id,
            appkit: NSRect::new(
                objc2_foundation::NSPoint::new(x, y),
                objc2_foundation::NSSize::new(800.0, 600.0),
            ),
            quartz: Rect {
                x,
                y,
                width: 800.0,
                height: 600.0,
            },
        };
        let mut screens = [
            display(3, -800.0, 0.0),
            display(1, 0.0, -600.0),
            display(2, 0.0, 0.0),
        ];
        screens.swap(0, 2);
        sort_displays(&mut screens);
        assert_eq!(screens.map(|display| display.id), [3, 1, 2]);
        assert_eq!(adjacent_display(&screens, 3, false), Some(screens[2]));
        assert_eq!(adjacent_display(&screens, 2, true), Some(screens[0]));
        assert_eq!(adjacent_display(&screens, 1, true), Some(screens[2]));
        assert_eq!(adjacent_display(&screens, 99, true), Some(screens[0]));
        assert_eq!(adjacent_display(&screens[..1], 3, true), Some(screens[0]));
        assert_eq!(adjacent_display(&[], 3, true), None);
    }
}
