//! Selection geometry uses top-left origins in logical screen points.

pub const MAX_DEPTH: usize = 9;
const MAX_GRID_DIVISIONS: u16 = 512;

/// Only powers of two can be reached by refining or merging the grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridDensity(u16);

impl Default for GridDensity {
    fn default() -> Self {
        Self(32)
    }
}

impl GridDensity {
    pub fn from_preference(value: isize) -> Self {
        u16::try_from(value)
            .ok()
            .filter(|value| value.is_power_of_two() && *value <= MAX_GRID_DIVISIONS)
            .map(Self)
            .unwrap_or_default()
    }

    pub fn divisions(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavigationMode {
    Quadtree,
    #[default]
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
    pub quadtree_can_subdivide: bool,
    pub destination_hints: [Option<DestinationHint>; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DestinationHint {
    pub key: char,
    pub point: Point,
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
        if !self.can_subdivide() {
            return;
        }
        if let Some(child) = self.current.quarter(quadrant) {
            self.parents.push(self.current);
            self.current = child;
        }
    }
    fn can_subdivide(&self) -> bool {
        self.parents.len() < MAX_DEPTH && self.current.quarter(Quadrant::TopLeft).is_some()
    }
    pub fn back(&mut self) {
        if let Some(parent) = self.parents.pop() {
            self.current = parent;
        }
    }
    fn select_at(&mut self, target: Point, depth: usize) {
        self.reset();
        for _ in 0..depth {
            let center = self.current.midpoint();
            let quadrant = match (target.x >= center.x, target.y >= center.y) {
                (false, false) => Quadrant::TopLeft,
                (true, false) => Quadrant::TopRight,
                (false, true) => Quadrant::BottomLeft,
                (true, true) => Quadrant::BottomRight,
            };
            self.descend(quadrant);
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

#[derive(Clone)]
struct GridSelection {
    root: Rect,
    state: GridState,
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
        };
        selection.center_on_grid(GridDensity::default().divisions());
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

    fn move_cell(&mut self, direction: Direction) {
        let last = self.state.divisions - 1;
        match direction {
            Direction::Left => self.state.column = self.state.column.saturating_sub(1),
            Direction::Down => self.state.row = self.state.row.saturating_add(1).min(last),
            Direction::Up => self.state.row = self.state.row.saturating_sub(1),
            Direction::Right => self.state.column = self.state.column.saturating_add(1).min(last),
        }
    }

    fn move_cells(&mut self, x: i16, y: i16) {
        let last = i32::from(self.state.divisions - 1);
        self.state.column = (i32::from(self.state.column) + i32::from(x)).clamp(0, last) as u16;
        self.state.row = (i32::from(self.state.row) + i32::from(y)).clamp(0, last) as u16;
    }
    fn resize(&mut self, refine: bool) {
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
    }

    fn reset(&mut self) {
        self.center_on_grid(2);
    }

    fn center_on_grid(&mut self, mut divisions: u16) {
        while self.root.width / f64::from(divisions) < 1.0
            || self.root.height / f64::from(divisions) < 1.0
        {
            divisions /= 2;
        }
        self.state.divisions = divisions;
        self.select_at(self.root.midpoint());
    }
}

/// Modes share a target area while keeping their own navigation resolution.
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

    /// New overlays start at the center; callers can restore the preferred density.
    pub fn new_in_mode(root: Rect, mode: NavigationMode) -> Self {
        let mut navigation = Self::new(root);
        navigation.mode = mode;
        navigation
    }

    pub fn with_grid_density(mut self, density: GridDensity) -> Self {
        self.grid.center_on_grid(density.divisions());
        self
    }

    pub fn grid_density(&self) -> GridDensity {
        GridDensity(self.grid.state.divisions)
    }

    pub fn current(&self) -> Rect {
        match self.mode {
            NavigationMode::Quadtree => self.quadtree.current(),
            NavigationMode::Grid => self.grid.current(),
        }
    }

    pub fn overlay_state(&self, show_hints: bool, stride: i16) -> OverlayState {
        OverlayState {
            selection: self.current(),
            root: self.root,
            mode: self.mode,
            grid_divisions: (self.mode == NavigationMode::Grid)
                .then_some(self.grid.state.divisions),
            quadtree_can_subdivide: self.mode == NavigationMode::Quadtree
                && self.quadtree.can_subdivide(),
            destination_hints: if show_hints {
                self.destination_hints(stride)
            } else {
                [None; 4]
            },
        }
    }

    /// Preview the same geometric transitions navigation uses, without changing
    /// the selection. Boundary no-ops have no useful destination to label.
    fn destination_hints(&self, stride: i16) -> [Option<DestinationHint>; 4] {
        let keys = ['h', 'j', 'k', 'l'];
        std::array::from_fn(|index| {
            let destination = match self.mode {
                NavigationMode::Quadtree => {
                    if !self.quadtree.can_subdivide() {
                        return None;
                    }
                    let quadrant = [
                        Quadrant::TopLeft,
                        Quadrant::BottomLeft,
                        Quadrant::BottomRight,
                        Quadrant::TopRight,
                    ][index];
                    self.quadtree.current().quarter(quadrant)?
                }
                NavigationMode::Grid => {
                    let mut preview = self.grid.clone();
                    for _ in 0..stride {
                        preview.move_cell(
                            [
                                Direction::Left,
                                Direction::Down,
                                Direction::Up,
                                Direction::Right,
                            ][index],
                        );
                    }
                    let destination = preview.current();
                    if destination == self.current() {
                        return None;
                    }
                    destination
                }
            };
            Some(DestinationHint {
                key: keys[index],
                point: destination.midpoint(),
            })
        })
    }

    pub fn switch_mode(&mut self, mode: NavigationMode) {
        if self.mode == mode {
            return;
        }
        let target = self.current().midpoint();
        match mode {
            NavigationMode::Grid => {
                self.grid.select_at(target);
            }
            NavigationMode::Quadtree => {
                let depth = if self.quadtree.parents.is_empty() {
                    self.grid.state.divisions.ilog2() as usize
                } else {
                    self.quadtree.parents.len()
                };
                self.quadtree.select_at(target, depth);
            }
        }
        self.mode = mode;
    }

    /// Keep each mode's relative target when moving to different display bounds.
    /// Resolution is reduced only when a cell would become smaller than a point.
    pub fn rebase(&mut self, root: Rect) {
        let map = |point: Point| Point {
            x: root.x + (point.x - self.root.x) / self.root.width * root.width,
            y: root.y + (point.y - self.root.y) / self.root.height * root.height,
        };
        let tree_target = map(self.quadtree.current().midpoint());
        let grid_target = map(self.grid.current().midpoint());
        let depth = self.quadtree.parents.len();
        self.quadtree = Selection::new(root);
        self.quadtree.select_at(tree_target, depth);
        self.grid.root = root;
        while root.width / f64::from(self.grid.state.divisions) < 1.0
            || root.height / f64::from(self.grid.state.divisions) < 1.0
        {
            self.grid.state.divisions /= 2;
        }
        self.grid.select_at(grid_target);
        self.root = root;
    }

    pub fn descend(&mut self, quadrant: Quadrant) {
        self.quadtree.descend(quadrant);
    }
    pub fn move_cell(&mut self, direction: Direction) {
        self.grid.move_cell(direction);
    }
    pub fn move_cells(&mut self, x: i16, y: i16) {
        self.grid.move_cells(x, y);
    }
    pub fn resize_grid(&mut self, refine: bool) {
        self.grid.resize(refine);
    }
    pub fn back(&mut self) {
        match self.mode {
            NavigationMode::Quadtree => self.quadtree.back(),
            NavigationMode::Grid => {}
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
    fn grid_stride_and_diagonal_geometry_match_control_hints() {
        let Some(root) = Rect::new(0.0, 0.0, 800.0, 800.0) else {
            panic!("valid fixture")
        };
        let mut nav = Navigation::new_in_mode(root, NavigationMode::Grid);
        for _ in 0..3 {
            nav.resize_grid(true);
        }
        nav.move_cells(-4, -4);
        let before = nav.current().midpoint();
        let hints = nav.overlay_state(true, 3).destination_hints;
        let Some(left_hint) = hints[0] else {
            panic!("left destination exists")
        };
        let left = left_hint.point;
        nav.move_cells(-3, 0);
        assert_eq!(nav.current().midpoint(), left);
        nav.move_cells(3, 3);
        assert_eq!(nav.current().midpoint().x, before.x);
        assert_eq!(
            nav.current().midpoint().y,
            before.y + nav.current().height * 3.0
        );
        nav.move_cells(i16::MAX, i16::MAX);
        let edge = nav.current();
        nav.move_cells(3, 3);
        assert_eq!(nav.current(), edge);
        nav.move_cells(i16::MIN, i16::MIN);
        assert_eq!(nav.current().x, 0.0);
        assert_eq!(nav.current().y, 0.0);
    }

    #[test]
    fn destination_hints_match_navigation_without_mutating_selection() {
        let Some(root) = Rect::new(-120.0, 30.0, 4096.0, 2048.0) else {
            panic!("valid fixture")
        };
        for mode in [NavigationMode::Quadtree, NavigationMode::Grid] {
            let mut navigation = Navigation::new_in_mode(root, mode);
            let before = navigation.current();
            assert_eq!(
                navigation.overlay_state(false, 1).quadtree_can_subdivide,
                mode == NavigationMode::Quadtree
            );
            assert_eq!(
                navigation.overlay_state(false, 1).destination_hints,
                [None; 4]
            );
            let hints = navigation.overlay_state(true, 1).destination_hints;
            assert_eq!(navigation.current(), before);
            for (index, hint) in hints.iter().enumerate() {
                let mut preview = Navigation::new_in_mode(root, mode);
                match mode {
                    NavigationMode::Quadtree => preview.descend(
                        [
                            Quadrant::TopLeft,
                            Quadrant::BottomLeft,
                            Quadrant::BottomRight,
                            Quadrant::TopRight,
                        ][index],
                    ),
                    NavigationMode::Grid => preview.move_cell(
                        [
                            Direction::Left,
                            Direction::Down,
                            Direction::Up,
                            Direction::Right,
                        ][index],
                    ),
                }
                assert_eq!(
                    *hint,
                    (preview.current() != before).then_some(DestinationHint {
                        key: ['h', 'j', 'k', 'l'][index],
                        point: preview.current().midpoint(),
                    })
                );
            }
            match mode {
                NavigationMode::Quadtree => {
                    for _ in 0..MAX_DEPTH {
                        navigation.descend(Quadrant::TopLeft);
                    }
                    assert_eq!(
                        navigation.overlay_state(true, 1).destination_hints,
                        [None; 4]
                    );
                    assert!(!navigation.overlay_state(false, 1).quadtree_can_subdivide);
                }
                NavigationMode::Grid => {
                    while navigation.grid_density().divisions() > 1 {
                        navigation.resize_grid(false);
                    }
                    assert_eq!(
                        navigation.overlay_state(true, 1).destination_hints,
                        [None; 4]
                    );
                    navigation.resize_grid(true);
                    let Some(destination) = Rect::new(0.0, 0.0, 8192.0, 4096.0) else {
                        panic!("valid fixture")
                    };
                    navigation.rebase(destination);
                    for hint in navigation
                        .overlay_state(true, 1)
                        .destination_hints
                        .into_iter()
                        .flatten()
                    {
                        assert!(destination.contains(hint.point));
                        assert_ne!(hint.point, navigation.current().midpoint());
                    }
                }
            }
        }
        let Some(tiny) = Rect::new(0.0, 0.0, 1.0, 1.0) else {
            panic!("valid fixture")
        };
        assert_eq!(
            Navigation::new(tiny)
                .overlay_state(true, 1)
                .destination_hints,
            [None; 4]
        );
        assert!(
            !Navigation::new_in_mode(tiny, NavigationMode::Quadtree)
                .overlay_state(false, 1)
                .quadtree_can_subdivide
        );
    }

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
    fn grid_movement_density_and_reset() {
        let Some(root) = Rect::new(-100.0, 20.0, 800.0, 600.0) else {
            panic!("valid fixture")
        };
        let mut navigation = Navigation::new(root).with_grid_density(GridDensity(2));
        navigation.switch_mode(NavigationMode::Grid);
        assert_eq!(
            navigation.grid.state,
            GridState {
                divisions: 2,
                column: 1,
                row: 1
            }
        );
        navigation.move_cell(Direction::Up);
        navigation.move_cell(Direction::Left);
        assert_eq!(navigation.grid.state.row, 0);
        assert_eq!(navigation.grid.state.column, 0);
        let target = navigation.current().midpoint();
        navigation.resize_grid(true);
        assert_eq!(navigation.grid.state.divisions, 4);
        assert!(navigation.current().contains(target));
        let refined = navigation.current();
        navigation.back();
        assert_eq!(navigation.current(), refined);
        for _ in 0..12 {
            navigation.move_cell(Direction::Up);
            navigation.move_cell(Direction::Left);
        }
        assert_eq!(navigation.grid.state.row, 0);
        assert_eq!(navigation.grid.state.column, 0);
        navigation.reset();
        assert_eq!(
            navigation.grid.state,
            GridState {
                divisions: 2,
                column: 1,
                row: 1
            }
        );
        assert!(navigation.current().contains(root.midpoint()));
        let reset = navigation.current();
        navigation.back();
        assert_eq!(navigation.current(), reset);
        for _ in 0..12 {
            navigation.resize_grid(false);
        }
        assert_eq!(navigation.grid.state.divisions, 1);
        assert_eq!(navigation.current(), root);
        navigation.resize_grid(false);
        assert_eq!(navigation.current(), root);
    }

    #[test]
    fn fresh_sessions_restore_density_and_mode_but_not_targets() {
        let Some(root) = Rect::new(0.0, 0.0, 1024.0, 768.0) else {
            panic!("valid fixture")
        };
        let mut previous = Navigation::new_in_mode(root, NavigationMode::Grid);
        assert_eq!(previous.grid_density().divisions(), 32);
        previous.resize_grid(true);
        previous.move_cells(-12, 8);
        assert_eq!(previous.grid_density().divisions(), 64);
        assert!(!previous.current().contains(root.midpoint()));
        let reopened = Navigation::new_in_mode(root, NavigationMode::Grid)
            .with_grid_density(previous.grid_density());
        assert_eq!(reopened.overlay_state(false, 1).grid_divisions, Some(64));
        assert!(reopened.current().contains(root.midpoint()));

        previous.switch_mode(NavigationMode::Quadtree);
        previous.descend(Quadrant::TopLeft);
        let mut reopened = Navigation::new_in_mode(root, NavigationMode::Quadtree)
            .with_grid_density(previous.grid_density());
        assert_eq!(reopened.current(), root);
        assert_eq!(reopened.overlay_state(false, 1).grid_divisions, None);
        reopened.switch_mode(NavigationMode::Grid);
        assert_eq!(reopened.overlay_state(false, 1).grid_divisions, Some(64));
        assert!(reopened.current().contains(root.midpoint()));
        reopened.reset();
        let reset = Navigation::new_in_mode(root, NavigationMode::Grid)
            .with_grid_density(reopened.grid_density());
        assert_eq!(reset.overlay_state(false, 1).grid_divisions, Some(2));

        for invalid in [isize::MIN, -1, 0, 3, 33, 513, 1024, isize::MAX] {
            assert_eq!(GridDensity::from_preference(invalid).divisions(), 32);
        }
        for valid in [1, 2, 32, 64, 512] {
            assert_eq!(
                GridDensity::from_preference(valid).divisions(),
                valid as u16
            );
        }
        let Some(small) = Rect::new(0.0, 0.0, 17.0, 9.0) else {
            panic!("valid fixture")
        };
        let clamped = Navigation::new_in_mode(small, NavigationMode::Grid)
            .with_grid_density(GridDensity::from_preference(512));
        assert_eq!(clamped.grid_density().divisions(), 8);
        assert!(clamped.current().width >= 1.0 && clamped.current().height >= 1.0);
    }

    #[test]
    fn grid_refinement_is_bounded() {
        let Some(root) = Rect::new(0.0, 0.0, 4096.0, 4096.0) else {
            panic!("valid fixture")
        };
        let mut grid = GridSelection::new(root);
        for _ in 0..12 {
            grid.resize(true);
        }
        assert_eq!(grid.state.divisions, 512);
        let Some(small) = Rect::new(0.0, 0.0, 3.0, 5.0) else {
            panic!("valid fixture")
        };
        let mut grid = GridSelection::new(small);
        assert_eq!(grid.state.divisions, 2);
        let last = grid.current();
        grid.resize(true);
        assert_eq!(grid.current(), last);
        let Some(tiny) = Rect::new(0.0, 0.0, 1.0, 1.0) else {
            panic!("valid fixture")
        };
        let mut grid = GridSelection::new(tiny);
        grid.reset();
        assert_eq!(grid.state.divisions, 1);
        assert_eq!(grid.current(), tiny);
    }

    #[test]
    fn mode_switches_keep_target_inside_receiving_cell() {
        let Some(root) = Rect::new(0.0, 0.0, 800.0, 600.0) else {
            panic!("valid fixture")
        };
        let mut navigation = Navigation::new(root).with_grid_density(GridDensity(2));
        navigation.switch_mode(NavigationMode::Grid);
        assert_eq!(navigation.overlay_state(false, 1).grid_divisions, Some(2));
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
        assert_eq!(navigation.quadtree.parents.len(), 2);
        navigation.reset();
        assert_eq!(navigation.current(), root);
    }

    #[test]
    fn display_transfer_preserves_modes_resolution_and_relative_targets() {
        let Some(root) = Rect::new(0.0, 0.0, 1024.0, 768.0) else {
            panic!("valid fixture")
        };
        let Some(destination) = Rect::new(0.0, 0.0, 2048.0, 1536.0) else {
            panic!("valid fixture")
        };
        let mut navigation = Navigation::new(root).with_grid_density(GridDensity(2));
        navigation.descend(Quadrant::TopRight);
        navigation.descend(Quadrant::BottomLeft);
        let target = navigation.current().midpoint();
        navigation.rebase(destination);
        assert_eq!(navigation.mode, NavigationMode::Quadtree);
        assert_eq!(navigation.quadtree.parents.len(), 2);
        assert_eq!(
            navigation.current().midpoint(),
            Point {
                x: target.x * 2.0,
                y: target.y * 2.0
            }
        );
        navigation.back();
        assert_eq!(navigation.current().width, 1024.0);
        navigation.switch_mode(NavigationMode::Grid);
        navigation.resize_grid(true);
        navigation.move_cell(Direction::Down);
        let target = navigation.current().midpoint();
        navigation.rebase(root);
        assert_eq!(navigation.mode, NavigationMode::Grid);
        assert_eq!(navigation.grid.state.divisions, 4);
        assert_eq!(
            navigation.current().midpoint(),
            Point {
                x: target.x / 2.0,
                y: target.y / 2.0
            }
        );
        let Some(tiny) = Rect::new(0.0, 0.0, 3.0, 2.0) else {
            panic!("valid fixture")
        };
        navigation.rebase(tiny);
        assert_eq!(navigation.grid.state.divisions, 2);
        assert!(navigation.current().width >= 1.0);
        assert!(navigation.current().height >= 1.0);
        navigation.switch_mode(NavigationMode::Quadtree);
        assert!(navigation.current().width >= 1.0);
        assert!(navigation.current().height >= 1.0);
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
