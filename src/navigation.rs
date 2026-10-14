//! Selection geometry uses top-left origins in logical screen points.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Option<Self> {
        if [x, y, width, height].iter().all(|v| v.is_finite())
            && width >= 1.0
            && height >= 1.0
            && (x + width).is_finite()
            && (y + height).is_finite()
        {
            Some(Self {
                x,
                y,
                width,
                height,
            })
        } else {
            None
        }
    }

    pub fn midpoint(self) -> Point {
        Point {
            x: self.x + self.width / 2.0,
            y: self.y + self.height / 2.0,
        }
    }

    pub fn contains(self, point: Point) -> bool {
        point.x >= self.x
            && point.x < self.x + self.width
            && point.y >= self.y
            && point.y < self.y + self.height
    }

    fn quarter(self, quadrant: Quadrant) -> Option<Self> {
        let width = self.width / 2.0;
        let height = self.height / 2.0;
        let x = self.x
            + if matches!(quadrant, Quadrant::TopRight | Quadrant::BottomRight) {
                width
            } else {
                0.0
            };
        let y = self.y
            + if matches!(quadrant, Quadrant::BottomLeft | Quadrant::BottomRight) {
                height
            } else {
                0.0
            };
        Self::new(x, y, width, height)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quadrant {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

pub struct Selection {
    root: Rect,
    current: Rect,
    parents: Vec<Rect>,
}

impl Selection {
    pub fn new(root: Rect) -> Self {
        Self {
            root,
            current: root,
            parents: Vec::new(),
        }
    }
    pub fn current(&self) -> Rect {
        self.current
    }
    pub fn descend(&mut self, quadrant: Quadrant) {
        if let Some(child) = self.current.quarter(quadrant) {
            self.parents.push(self.current);
            self.current = child;
        }
    }
    pub fn back(&mut self) {
        if let Some(parent) = self.parents.pop() {
            self.current = parent;
        }
    }
    pub fn reset(&mut self) {
        self.parents.clear();
        self.current = self.root;
    }
}

/// Map local overlay points to Quartz's top-left global event coordinates.
/// Quartz bounds and AppKit frames can have different extents on scaled displays.
pub fn event_point(local: Point, local_size: (f64, f64), quartz: Rect) -> Point {
    Point {
        x: quartz.x + local.x / local_size.0 * quartz.width,
        y: quartz.y + local.y / local_size.1 * quartz.height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn path_parent_reset_and_click_midpoint() {
        let Some(root) = Rect::new(0.0, 0.0, 800.0, 600.0) else {
            panic!("valid fixture")
        };
        let mut selection = Selection::new(root);
        selection.descend(Quadrant::TopRight);
        selection.descend(Quadrant::BottomLeft);
        assert_eq!(selection.current().midpoint(), Point { x: 500.0, y: 225.0 });
        selection.back();
        assert_eq!(
            selection.current(),
            Rect {
                x: 400.0,
                y: 0.0,
                width: 400.0,
                height: 300.0
            }
        );
        selection.reset();
        selection.back();
        assert_eq!(selection.current(), root);
    }
    #[test]
    fn finite_bounds_and_subdivision_floor() {
        for dimensions in [
            (f64::NAN, 1.0),
            (1.0, f64::INFINITY),
            (0.0, 1.0),
            (-1.0, 1.0),
        ] {
            assert!(Rect::new(0.0, 0.0, dimensions.0, dimensions.1).is_none());
        }
        let Some(root) = Rect::new(0.0, 0.0, 3.0, 3.0) else {
            panic!("valid fixture")
        };
        let mut selection = Selection::new(root);
        selection.descend(Quadrant::BottomRight);
        let last = selection.current();
        selection.descend(Quadrant::TopLeft);
        assert_eq!(selection.current(), last);
        assert_eq!(last.width, 1.5);
    }
    #[test]
    fn coordinate_mapping_negative_origin_and_scaled_display() {
        let Some(quartz) = Rect::new(-1920.0, -200.0, 1920.0, 1080.0) else {
            panic!("valid fixture")
        };
        assert_eq!(
            event_point(Point { x: 480.0, y: 270.0 }, (960.0, 540.0), quartz),
            Point {
                x: -960.0,
                y: 340.0
            }
        );
        assert!(quartz.contains(Point {
            x: -1920.0,
            y: -200.0
        }));
        assert!(!quartz.contains(Point { x: 0.0, y: 0.0 }));
    }
}
