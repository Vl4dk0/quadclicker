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
