//! Selection geometry uses top-left origins in logical screen points.

pub const MAX_DEPTH: usize = 9;
const MAX_GRID_DIVISIONS: u16 = 512;
const GRID_HISTORY_LIMIT: usize = 256;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavigationMode {
    #[default]
    Quadtree,
    Grid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Down,
    Up,
    Right,
}

#[derive(Clone, Copy, Debug)]
pub struct OverlayState {
    pub selection: Rect,
    pub root: Rect,
    pub mode: NavigationMode,
    pub grid_divisions: Option<u16>,
}

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
        if self.parents.len() >= MAX_DEPTH {
            return;
        }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GridState {
    divisions: u16,
    column: u16,
    row: u16,
}

struct GridSelection {
    root: Rect,
    state: GridState,
    history: std::collections::VecDeque<GridState>,
}

impl GridSelection {
    fn new(root: Rect) -> Self {
        let mut selection = Self {
            root,
            state: GridState {
                divisions: 1,
                column: 0,
                row: 0,
            },
            history: std::collections::VecDeque::new(),
        };
        selection.reset();
        selection
    }

    fn current(&self) -> Rect {
        let width = self.root.width / f64::from(self.state.divisions);
        let height = self.root.height / f64::from(self.state.divisions);
        Rect {
            x: self.root.x + f64::from(self.state.column) * width,
            y: self.root.y + f64::from(self.state.row) * height,
            width,
            height,
        }
    }

    fn select_at(&mut self, point: Point) {
        // Geometry is already finite; clamp boundary points to the last cell.
        let divisions = f64::from(self.state.divisions);
        let last = divisions - 1.0;
        self.state.column = ((point.x - self.root.x) / self.root.width * divisions)
            .floor()
            .clamp(0.0, last) as u16;
        self.state.row = ((point.y - self.root.y) / self.root.height * divisions)
            .floor()
            .clamp(0.0, last) as u16;
    }

    fn remember_change(&mut self, previous: GridState) {
        if self.state == previous {
            return;
        }
        // Undo remains bounded even during a long navigation session.
        if self.history.len() == GRID_HISTORY_LIMIT {
            self.history.pop_front();
        }
        self.history.push_back(previous);
    }

    fn move_cell(&mut self, direction: Direction) {
        let previous = self.state;
        let last = self.state.divisions - 1;
        match direction {
            Direction::Left => self.state.column = self.state.column.saturating_sub(1),
            Direction::Down => self.state.row = self.state.row.saturating_add(1).min(last),
            Direction::Up => self.state.row = self.state.row.saturating_sub(1),
            Direction::Right => self.state.column = self.state.column.saturating_add(1).min(last),
        }
        self.remember_change(previous);
    }

    fn resize(&mut self, refine: bool) {
        let previous = self.state;
        let divisions = if refine {
            if self.state.divisions >= MAX_GRID_DIVISIONS {
                return;
            }
            self.state.divisions * 2
        } else {
            (self.state.divisions / 2).max(1)
        };
        if self.root.width / f64::from(divisions) < 1.0
            || self.root.height / f64::from(divisions) < 1.0
        {
            return;
        }
        let target = self.current().midpoint();
        self.state.divisions = divisions;
        self.select_at(target);
        self.remember_change(previous);
    }

    fn back(&mut self) {
        if let Some(previous) = self.history.pop_back() {
            self.state = previous;
        }
    }

    fn reset(&mut self) {
        let mut divisions = 4;
        while self.root.width / f64::from(divisions) < 1.0
            || self.root.height / f64::from(divisions) < 1.0
        {
            divisions /= 2;
        }
        self.state.divisions = divisions;
        self.select_at(self.root.midpoint());
        self.history.clear();
    }
}

/// Modes share a target area while keeping their own navigation resolution.
/// Switching establishes a new undo origin instead of restoring stale movement.
pub struct Navigation {
    root: Rect,
    mode: NavigationMode,
    quadtree: Selection,
    grid: GridSelection,
}

impl Navigation {
    pub fn new(root: Rect) -> Self {
        Self {
            root,
            mode: NavigationMode::Quadtree,
            quadtree: Selection::new(root),
            grid: GridSelection::new(root),
        }
    }

    pub fn current(&self) -> Rect {
        match self.mode {
            NavigationMode::Quadtree => self.quadtree.current(),
            NavigationMode::Grid => self.grid.current(),
        }
    }

    pub fn overlay_state(&self) -> OverlayState {
        OverlayState {
            selection: self.current(),
            root: self.root,
            mode: self.mode,
            grid_divisions: (self.mode == NavigationMode::Grid)
                .then_some(self.grid.state.divisions),
        }
    }

    pub fn switch_mode(&mut self, mode: NavigationMode) {
        if self.mode == mode {
            return;
        }
        let target = self.current().midpoint();
        match mode {
            NavigationMode::Grid => {
                self.grid.select_at(target);
                self.grid.history.clear();
            }
            NavigationMode::Quadtree => {
                let depth = if self.quadtree.parents.is_empty() {
                    self.grid.state.divisions.ilog2() as usize
                } else {
                    self.quadtree.parents.len()
                };
                self.quadtree.reset();
                for _ in 0..depth {
                    let center = self.quadtree.current().midpoint();
                    let quadrant = match (target.x >= center.x, target.y >= center.y) {
                        (false, false) => Quadrant::TopLeft,
                        (true, false) => Quadrant::TopRight,
                        (false, true) => Quadrant::BottomLeft,
                        (true, true) => Quadrant::BottomRight,
                    };
                    self.quadtree.descend(quadrant);
                }
            }
        }
        self.mode = mode;
    }

    pub fn descend(&mut self, quadrant: Quadrant) {
        self.quadtree.descend(quadrant);
    }
    pub fn move_cell(&mut self, direction: Direction) {
        self.grid.move_cell(direction);
    }
    pub fn resize_grid(&mut self, refine: bool) {
        self.grid.resize(refine);
    }
    pub fn back(&mut self) {
        match self.mode {
            NavigationMode::Quadtree => self.quadtree.back(),
            NavigationMode::Grid => self.grid.back(),
        }
    }
    pub fn reset(&mut self) {
        match self.mode {
            NavigationMode::Quadtree => self.quadtree.reset(),
            NavigationMode::Grid => self.grid.reset(),
        }
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
    fn finite_bounds_and_navigation_limits() {
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

        // Large bounds keep the point-size floor out of the depth-limit case.
        let Some(root) = Rect::new(0.0, 0.0, 4096.0, 4096.0) else {
            panic!("valid fixture")
        };
        let mut selection = Selection::new(root);
        for _ in 0..9 {
            selection.descend(Quadrant::TopLeft);
        }
        let ninth = selection.current();
        assert_eq!(ninth.width, 8.0);
        selection.descend(Quadrant::BottomRight);
        assert_eq!(selection.current(), ninth);
        selection.back();
        assert_eq!(selection.current().width, 16.0);
        selection.descend(Quadrant::BottomRight);
        assert_eq!(
            selection.current(),
            Rect {
                x: 8.0,
                y: 8.0,
                width: 8.0,
                height: 8.0
            }
        );
        selection.reset();
        assert_eq!(selection.current(), root);
        for _ in 0..9 {
            selection.descend(Quadrant::TopLeft);
        }
        assert_eq!(selection.current(), ninth);
    }
    #[test]
    fn grid_movement_density_undo_and_reset() {
        let Some(root) = Rect::new(-100.0, 20.0, 800.0, 600.0) else {
            panic!("valid fixture")
        };
        let mut grid = GridSelection::new(root);
        assert_eq!(
            grid.state,
            GridState {
                divisions: 4,
                column: 2,
                row: 2
            }
        );
        let initial = grid.current();
        grid.move_cell(Direction::Up);
        grid.move_cell(Direction::Left);
        assert_eq!(grid.state.row, 1);
        assert_eq!(grid.state.column, 1);
        let target = grid.current().midpoint();
        grid.resize(true);
        assert_eq!(grid.state.divisions, 8);
        assert!(grid.current().contains(target));
        grid.back();
        assert_eq!(grid.state.divisions, 4);
        grid.back();
        assert_eq!(grid.state.column, 2);
        grid.back();
        assert_eq!(grid.current(), initial);
        grid.move_cell(Direction::Up);
        grid.move_cell(Direction::Up);
        let changes = grid.history.len();
        grid.move_cell(Direction::Up);
        assert_eq!(grid.history.len(), changes);
        grid.reset();
        assert_eq!(grid.current(), initial);
        grid.back();
        assert_eq!(grid.current(), initial);
        for _ in 0..12 {
            grid.resize(false);
        }
        assert_eq!(grid.state.divisions, 1);
        assert_eq!(grid.current(), root);
        let changes = grid.history.len();
        grid.resize(false);
        assert_eq!(grid.history.len(), changes);
    }

    #[test]
    fn grid_refinement_and_history_are_bounded() {
        let Some(root) = Rect::new(0.0, 0.0, 4096.0, 4096.0) else {
            panic!("valid fixture")
        };
        let mut grid = GridSelection::new(root);
        for _ in 0..12 {
            grid.resize(true);
        }
        assert_eq!(grid.state.divisions, 512);
        for _ in 0..300 {
            grid.move_cell(Direction::Left);
            grid.move_cell(Direction::Right);
        }
        assert_eq!(grid.history.len(), GRID_HISTORY_LIMIT);
        let Some(small) = Rect::new(0.0, 0.0, 3.0, 5.0) else {
            panic!("valid fixture")
        };
        let mut grid = GridSelection::new(small);
        assert_eq!(grid.state.divisions, 2);
        let last = grid.current();
        grid.resize(true);
        assert_eq!(grid.current(), last);
    }

    #[test]
    fn mode_switches_keep_target_inside_receiving_cell() {
        let Some(root) = Rect::new(0.0, 0.0, 800.0, 600.0) else {
            panic!("valid fixture")
        };
        let mut navigation = Navigation::new(root);
        navigation.switch_mode(NavigationMode::Grid);
        assert_eq!(navigation.overlay_state().grid_divisions, Some(4));
        navigation.move_cell(Direction::Left);
        navigation.move_cell(Direction::Up);
        let grid = navigation.current();
        navigation.switch_mode(NavigationMode::Quadtree);
        assert_eq!(navigation.current(), grid);
        navigation.descend(Quadrant::TopRight);
        let target = navigation.current().midpoint();
        navigation.switch_mode(NavigationMode::Grid);
        assert!(navigation.current().contains(target));
        let switched = navigation.current();
        navigation.back();
        assert_eq!(navigation.current(), switched);
        navigation.move_cell(Direction::Right);
        let target = navigation.current().midpoint();
        navigation.switch_mode(NavigationMode::Quadtree);
        assert!(navigation.current().contains(target));
        assert_eq!(navigation.quadtree.parents.len(), 3);
        navigation.reset();
        assert_eq!(navigation.current(), root);
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
