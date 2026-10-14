//! Physical key ownership ensures an intercepted press never leaks its release.
use crate::navigation::{Direction, NavigationMode, Quadrant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Descend(Quadrant),
    Move(Direction),
    ResizeGrid { refine: bool },
    SwitchMode(NavigationMode),
    Back,
    Reset,
    Click,
    Exit,
    Dismiss,
    Activate,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub shortcut: bool,
    pub function: bool,
}

pub struct KeyRouter {
    active: bool,
    mode: NavigationMode,
    held: [bool; 256],
}

impl Default for KeyRouter {
    fn default() -> Self {
        Self {
            active: false,
            mode: NavigationMode::Quadtree,
            held: [false; 256],
        }
    }
}

impl KeyRouter {
    pub fn activate(&mut self) {
        self.active = true;
        self.mode = NavigationMode::Quadtree;
    }
    pub fn deactivate(&mut self) {
        self.active = false;
    }
    pub fn interception_lost(&mut self) {
        // A disabled tap cannot observe releases. Its replacement must not
        // inherit presses whose corresponding release may already be gone.
        self.active = false;
        self.held.fill(false);
    }
    pub fn is_active(&self) -> bool {
        self.active
    }
    #[cfg(test)]
    pub fn is_drained(&self) -> bool {
        !self.held.iter().any(|held| *held)
    }
    pub fn release(&mut self, code: usize) -> bool {
        let Some(held) = self.held.get_mut(code) else {
            return false;
        };
        std::mem::replace(held, false)
    }
    pub fn modifier_changed(&mut self, code: usize, pressed: bool) -> Option<Command> {
        // Fn stays available as an activation-chord prefix, just as Shift stays
        // available for reset. Modifiers pass through to preserve target state.
        if !self.active || !pressed || matches!(code, 56 | 60 | 63) {
            return None;
        }
        self.active = false;
        Some(Command::Dismiss)
    }

    fn toggle(&mut self) -> Command {
        if self.active {
            self.active = false;
            Command::Exit
        } else {
            Command::Activate
        }
    }

    pub fn press(
        &mut self,
        code: usize,
        modifiers: Modifiers,
        repeat: bool,
    ) -> (bool, Option<Command>) {
        let Some(held) = self.held.get_mut(code) else {
            return (false, None);
        };
        if *held {
            return (true, None);
        }
        let activation_key = code == 160 || (code == 49 && modifiers.function);
        if activation_key && !modifiers.shortcut && !modifiers.shift && !repeat {
            *held = true;
            return (true, Some(self.toggle()));
        }
        if !self.active {
            return (false, None);
        }
        if repeat {
            // A press that began outside QuadClicker still belongs to the target
            // app. Its repeats dismiss navigation instead of becoming commands.
            self.active = false;
            return (false, Some(Command::Dismiss));
        }
        // Native keypad Enter (76) can carry Function metadata; it remains an
        // Enter action, including the OS mapping of Fn+Return to that key code.
        let command = if code == 53 {
            Some(Command::Exit)
        } else if modifiers.shortcut || (modifiers.function && code != 76) {
            None
        } else {
            match code {
                46 if !modifiers.shift => {
                    // Commands drain later. Update routing now so the next key
                    // already uses the mode requested by this physical press.
                    self.mode = match self.mode {
                        NavigationMode::Quadtree => NavigationMode::Grid,
                        NavigationMode::Grid => NavigationMode::Quadtree,
                    };
                    Some(Command::SwitchMode(self.mode))
                }
                31 | 51 if modifiers.shift => Some(Command::Reset),
                31 | 51 => Some(Command::Back),
                36 | 76 if !modifiers.shift => Some(Command::Click),
                _ if modifiers.shift => None,
                _ => match (self.mode, code) {
                    (NavigationMode::Quadtree, 32) => Some(Command::Descend(Quadrant::TopLeft)),
                    (NavigationMode::Quadtree, 34) => Some(Command::Descend(Quadrant::TopRight)),
                    (NavigationMode::Quadtree, 38) => Some(Command::Descend(Quadrant::BottomLeft)),
                    (NavigationMode::Quadtree, 40) => Some(Command::Descend(Quadrant::BottomRight)),
                    (NavigationMode::Grid, 4) => Some(Command::Move(Direction::Left)),
                    (NavigationMode::Grid, 38) => Some(Command::Move(Direction::Down)),
                    (NavigationMode::Grid, 40) => Some(Command::Move(Direction::Up)),
                    (NavigationMode::Grid, 37) => Some(Command::Move(Direction::Right)),
                    (NavigationMode::Grid, 32) => Some(Command::ResizeGrid { refine: false }),
                    (NavigationMode::Grid, 34) => Some(Command::ResizeGrid { refine: true }),
                    _ => None,
                },
            }
        };
        if let Some(command) = command {
            *held = true;
            if command == Command::Exit {
                self.active = false;
            }
            (true, Some(command))
        } else {
            // Preserve the original key event. Only navigation commands acquire
            // key ownership; ordinary typing and shortcuts return to the target.
            self.active = false;
            (false, Some(Command::Dismiss))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cluster_modifiers_and_single_press_policy() {
        let mut router = KeyRouter::default();
        router.activate();
        for (code, quadrant) in [
            (32, Quadrant::TopLeft),
            (34, Quadrant::TopRight),
            (38, Quadrant::BottomLeft),
            (40, Quadrant::BottomRight),
        ] {
            assert_eq!(
                router.press(code, Modifiers::default(), false),
                (true, Some(Command::Descend(quadrant)))
            );
            assert_eq!(router.press(code, Modifiers::default(), true), (true, None));
            assert!(router.release(code));
        }
        for code in [31, 51] {
            for (shift, command) in [(false, Command::Back), (true, Command::Reset)] {
                let modifiers = Modifiers {
                    shift,
                    ..Modifiers::default()
                };
                assert_eq!(router.press(code, modifiers, false), (true, Some(command)));
                assert!(router.is_active());
                assert_eq!(router.press(code, modifiers, true), (true, None));
                assert!(router.release(code));
            }
        }
        assert_eq!(
            router.press(
                36,
                Modifiers {
                    shift: true,
                    shortcut: false,
                    function: false
                },
                false
            ),
            (false, Some(Command::Dismiss))
        );
        assert!(!router.is_active());
        router.activate();
        assert_eq!(
            router.press(
                32,
                Modifiers {
                    shift: false,
                    shortcut: true,
                    function: false
                },
                false
            ),
            (false, Some(Command::Dismiss))
        );
        assert!(!router.is_active());
        router.activate();
        for code in [31, 51] {
            for shift in [false, true] {
                router.activate();
                assert_eq!(
                    router.press(
                        code,
                        Modifiers {
                            shift,
                            shortcut: true,
                            function: false
                        },
                        false
                    ),
                    (false, Some(Command::Dismiss))
                );
                assert!(!router.is_active());
                assert!(!router.release(code));
            }
        }
    }
    #[test]
    fn mode_switch_routes_immediately_and_preserves_key_ownership() {
        let mut router = KeyRouter::default();
        router.activate();
        assert_eq!(
            router.press(46, Modifiers::default(), false),
            (true, Some(Command::SwitchMode(NavigationMode::Grid)))
        );
        assert_eq!(router.press(46, Modifiers::default(), true), (true, None));
        assert_eq!(
            router.press(4, Modifiers::default(), false),
            (true, Some(Command::Move(Direction::Left)))
        );
        assert!(router.release(4));
        assert!(router.release(46));
        for (code, command) in [
            (38, Command::Move(Direction::Down)),
            (40, Command::Move(Direction::Up)),
            (37, Command::Move(Direction::Right)),
            (32, Command::ResizeGrid { refine: false }),
            (34, Command::ResizeGrid { refine: true }),
        ] {
            assert_eq!(
                router.press(code, Modifiers::default(), false),
                (true, Some(command))
            );
            assert!(router.release(code));
        }
        // Grid back is a no-op, but its full physical press remains owned.
        for code in [31, 51] {
            for (shift, command) in [(false, Command::Back), (true, Command::Reset)] {
                let modifiers = Modifiers {
                    shift,
                    ..Modifiers::default()
                };
                assert_eq!(router.press(code, modifiers, false), (true, Some(command)));
                assert!(router.is_active());
                assert_eq!(router.press(code, modifiers, true), (true, None));
                assert!(router.release(code));
            }
        }
        assert_eq!(
            router.press(46, Modifiers::default(), false),
            (true, Some(Command::SwitchMode(NavigationMode::Quadtree)))
        );
        assert!(router.release(46));
        assert_eq!(
            router.press(4, Modifiers::default(), false),
            (false, Some(Command::Dismiss))
        );
        assert!(!router.release(4));
        router.activate();
        assert_eq!(
            router.press(40, Modifiers::default(), false),
            (true, Some(Command::Descend(Quadrant::BottomRight)))
        );
        assert!(router.release(40));
        assert_eq!(
            router.press(
                4,
                Modifiers {
                    shift: true,
                    ..Modifiers::default()
                },
                false
            ),
            (false, Some(Command::Dismiss))
        );
    }

    #[test]
    fn escape_and_changed_modifiers_keep_paired_releases_owned() {
        let mut router = KeyRouter::default();
        router.activate();
        assert!(router.press(36, Modifiers::default(), false).0);
        assert_eq!(
            router.press(
                53,
                Modifiers {
                    shift: false,
                    shortcut: true,
                    function: false
                },
                false
            ),
            (true, Some(Command::Exit))
        );
        assert!(!router.is_active());
        assert_eq!(
            router.press(
                36,
                Modifiers {
                    shift: true,
                    shortcut: true,
                    function: false
                },
                true
            ),
            (true, None)
        );
        assert!(router.release(36));
        assert!(router.release(53));
        assert!(router.is_drained());
        assert!(!router.press(32, Modifiers::default(), false).0);
    }
    #[test]
    fn activation_aliases_toggle_and_own_full_presses() {
        let mut router = KeyRouter::default();
        assert_eq!(
            router.press(160, Modifiers::default(), false),
            (true, Some(Command::Activate))
        );
        assert_eq!(router.press(160, Modifiers::default(), true), (true, None));
        assert!(router.release(160));
        router.activate();
        assert_eq!(
            router.press(160, Modifiers::default(), false),
            (true, Some(Command::Exit))
        );
        assert!(!router.is_active());
        assert_eq!(router.press(160, Modifiers::default(), true), (true, None));
        assert!(router.release(160));
        assert!(
            !router
                .press(
                    160,
                    Modifiers {
                        shift: false,
                        shortcut: true,
                        function: false
                    },
                    false
                )
                .0
        );
        assert!(!router.press(99, Modifiers::default(), false).0);

        let fn_space = Modifiers {
            function: true,
            ..Modifiers::default()
        };
        assert_eq!(router.modifier_changed(63, true), None);
        assert_eq!(
            router.press(49, fn_space, false),
            (true, Some(Command::Activate))
        );
        router.activate();
        assert_eq!(router.modifier_changed(63, false), None);
        assert_eq!(router.press(49, Modifiers::default(), true), (true, None));
        assert!(router.release(49));
        assert_eq!(router.modifier_changed(63, true), None);
        assert!(router.is_active());
        assert_eq!(
            router.press(49, fn_space, false),
            (true, Some(Command::Exit))
        );
        assert!(router.release(49));
        assert_eq!(
            router.press(160, fn_space, false),
            (true, Some(Command::Activate))
        );
        assert!(router.release(160));
        for disallowed in [
            Modifiers {
                function: true,
                shift: true,
                shortcut: false,
            },
            Modifiers {
                function: true,
                shift: false,
                shortcut: true,
            },
        ] {
            assert_eq!(router.press(49, disallowed, false), (false, None));
            assert!(!router.release(49));
            router.activate();
            assert_eq!(
                router.press(49, disallowed, false),
                (false, Some(Command::Dismiss))
            );
            assert!(!router.is_active());
            assert!(!router.release(49));
        }
        assert_eq!(router.press(49, Modifiers::default(), false), (false, None));
        assert_eq!(router.modifier_changed(63, true), None);
        assert_eq!(router.press(49, fn_space, true), (false, None));
        assert!(!router.release(49));
        router.activate();
        assert_eq!(
            router.press(49, fn_space, true),
            (false, Some(Command::Dismiss))
        );
        assert!(!router.is_active());
        assert!(!router.release(49));
    }
    #[test]
    fn ordinary_typing_and_unowned_repeats_dismiss_without_acquiring_key_ownership() {
        let mut router = KeyRouter::default();
        assert_eq!(router.press(32, Modifiers::default(), false), (false, None));
        router.activate();
        assert_eq!(
            router.press(32, Modifiers::default(), true),
            (false, Some(Command::Dismiss))
        );
        assert!(!router.is_active());
        assert!(!router.release(32));
        router.activate();
        assert!(router.press(38, Modifiers::default(), false).0);
        assert_eq!(
            router.press(0, Modifiers::default(), false),
            (false, Some(Command::Dismiss))
        );
        assert!(!router.is_active());
        assert!(!router.release(0));
        assert!(router.release(38));
        assert!(!router.release(300));

        for code in [56, 60, 63] {
            router.activate();
            assert_eq!(router.modifier_changed(code, true), None);
            assert_eq!(router.modifier_changed(code, false), None);
            assert!(router.is_active());
        }
        let function = Modifiers {
            function: true,
            ..Modifiers::default()
        };
        for code in [32, 34, 38, 40, 31, 51, 36] {
            router.activate();
            router.modifier_changed(63, true);
            assert_eq!(
                router.press(code, function, false),
                (false, Some(Command::Dismiss))
            );
            assert!(!router.release(code));
        }
        router.modifier_changed(63, false);
        router.activate();
        assert_eq!(
            router.press(76, function, false),
            (true, Some(Command::Click))
        );
        assert!(router.release(76));
        assert_eq!(
            router.press(49, Modifiers::default(), false),
            (false, Some(Command::Dismiss))
        );
        for code in [54, 55, 58, 61, 59, 62, 57] {
            router.activate();
            assert_eq!(router.modifier_changed(code, false), None);
            assert!(router.is_active());
            assert_eq!(router.modifier_changed(code, true), Some(Command::Dismiss));
            assert!(!router.is_active());
        }
    }
}
