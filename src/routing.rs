//! Physical key ownership ensures an intercepted press never leaks its release.
use crate::navigation::{Direction, NavigationMode, Quadrant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Descend(Quadrant),
    Move(Direction),
    ResizeGrid { refine: bool },
    SwitchMode(NavigationMode),
    ShowHints(bool),
    PreviousDisplay,
    NextDisplay,
    Back,
    Reset,
    Click,
    Exit,
    Dismiss,
    Activate,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub shortcut: bool,
    pub function: bool,
}

#[derive(Clone, Copy)]
struct OwnedPress {
    modifiers: Modifiers,
    repeat_action: Option<Command>,
}

pub struct KeyRouter {
    active: bool,
    mode: NavigationMode,
    show_hints: bool,
    held: [Option<OwnedPress>; 256],
}

impl Default for KeyRouter {
    fn default() -> Self {
        Self {
            active: false,
            mode: NavigationMode::default(),
            show_hints: false,
            held: [None; 256],
        }
    }
}

impl KeyRouter {
    pub fn activate(&mut self) {
        self.cancel_repeats();
        self.active = true;
    }
    pub fn mode(&self) -> NavigationMode {
        self.mode
    }
    pub fn show_hints(&self) -> bool {
        self.show_hints
    }
    pub fn set_show_hints(&mut self, show: bool) {
        self.show_hints = show;
    }
    pub fn set_mode(&mut self, mode: NavigationMode) {
        self.cancel_repeats();
        self.mode = mode;
    }
    pub fn deactivate(&mut self) {
        self.cancel_repeats();
        self.active = false;
    }
    pub fn interception_lost(&mut self) {
        // A disabled tap cannot observe releases. Its replacement must not
        // inherit presses whose corresponding release may already be gone.
        self.active = false;
        self.held.fill(None);
    }
    pub fn is_active(&self) -> bool {
        self.active
    }
    #[cfg(test)]
    pub fn is_drained(&self) -> bool {
        !self.held.iter().any(Option::is_some)
    }
    pub fn release(&mut self, code: usize) -> bool {
        let Some(held) = self.held.get_mut(code) else {
            return false;
        };
        held.take().is_some()
    }
    fn cancel_repeats(&mut self) {
        for press in self.held.iter_mut().flatten() {
            press.repeat_action = None;
        }
    }
    pub fn modifier_changed(&mut self, code: usize, pressed: bool) -> Option<Command> {
        self.cancel_repeats();
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
            self.deactivate();
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
        if let Some(press) = held {
            // Ownership lasts through key-up, but the original action can only
            // repeat within its uninterrupted mode, session and modifiers.
            if modifiers != press.modifiers || !self.active {
                press.repeat_action = None;
            }
            return (true, if repeat { press.repeat_action } else { None });
        }
        let activation_key = code == 160 || (code == 49 && modifiers.function);
        if activation_key && !modifiers.shortcut && !modifiers.shift && !repeat {
            *held = Some(OwnedPress {
                modifiers,
                repeat_action: None,
            });
            return (true, Some(self.toggle()));
        }
        if !self.active {
            return (false, None);
        }
        if repeat {
            // A press that began outside QuadClicker still belongs to the target
            // app. Its repeats dismiss navigation instead of becoming commands.
            self.deactivate();
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
                    self.cancel_repeats();
                    self.mode = match self.mode {
                        NavigationMode::Quadtree => NavigationMode::Grid,
                        NavigationMode::Grid => NavigationMode::Quadtree,
                    };
                    Some(Command::SwitchMode(self.mode))
                }
                17 if !modifiers.shift => {
                    self.show_hints = !self.show_hints;
                    Some(Command::ShowHints(self.show_hints))
                }
                38 if modifiers.shift && self.mode == NavigationMode::Grid => {
                    Some(Command::ResizeGrid { refine: false })
                }
                40 if modifiers.shift && self.mode == NavigationMode::Grid => {
                    Some(Command::ResizeGrid { refine: true })
                }
                4 if modifiers.shift => Some(Command::PreviousDisplay),
                37 if modifiers.shift => Some(Command::NextDisplay),
                31 | 51 if modifiers.shift => Some(Command::Reset),
                31 | 51 => Some(Command::Back),
                36 | 76 if !modifiers.shift => Some(Command::Click),
                _ if modifiers.shift => None,
                _ => match (self.mode, code) {
                    (NavigationMode::Quadtree, 4) => Some(Command::Descend(Quadrant::TopLeft)),
                    (NavigationMode::Quadtree, 37) => Some(Command::Descend(Quadrant::TopRight)),
                    (NavigationMode::Quadtree, 38) => Some(Command::Descend(Quadrant::BottomLeft)),
                    (NavigationMode::Quadtree, 40) => Some(Command::Descend(Quadrant::BottomRight)),
                    (NavigationMode::Grid, 4) => Some(Command::Move(Direction::Left)),
                    (NavigationMode::Grid, 38) => Some(Command::Move(Direction::Down)),
                    (NavigationMode::Grid, 40) => Some(Command::Move(Direction::Up)),
                    (NavigationMode::Grid, 37) => Some(Command::Move(Direction::Right)),
                    _ => None,
                },
            }
        };
        if let Some(command) = command {
            let repeat_action = match command {
                Command::Descend(_) | Command::Move(_) | Command::ResizeGrid { .. } => {
                    Some(command)
                }
                Command::Back if self.mode == NavigationMode::Quadtree => Some(command),
                _ => None,
            };
            self.held[code] = Some(OwnedPress {
                modifiers,
                repeat_action,
            });
            if command == Command::Exit {
                self.deactivate();
            }
            (true, Some(command))
        } else {
            // A dismissal key belongs to the overlay, just like Escape. Sending
            // it to the target can type unwanted text or trigger its error beep.
            self.held[code] = Some(OwnedPress {
                modifiers,
                repeat_action: None,
            });
            self.deactivate();
            (true, Some(Command::Dismiss))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_mode_hints_and_removed_controls() {
        let mut router = KeyRouter::default();
        assert_eq!(router.mode(), NavigationMode::Grid);
        assert!(!router.show_hints());
        let plain = Modifiers::default();
        router.activate();
        assert_eq!(
            router.press(17, plain, false),
            (true, Some(Command::ShowHints(true)))
        );
        assert!(router.show_hints());
        assert_eq!(router.press(17, plain, true), (true, None));
        // A dismissal may cancel deferred rendering but not the preference.
        assert_eq!(
            router.press(0, plain, false),
            (true, Some(Command::Dismiss))
        );
        assert!(router.show_hints());
        assert!(router.release(0));
        assert!(router.release(17));
        router.activate();
        assert_eq!(
            router.press(17, plain, false),
            (true, Some(Command::ShowHints(false)))
        );
        assert!(router.release(17));
        for mode in [NavigationMode::Quadtree, NavigationMode::Grid] {
            router.set_mode(mode);
            for code in [32, 34] {
                router.activate();
                assert_eq!(
                    router.press(code, plain, false),
                    (true, Some(Command::Dismiss))
                );
                assert!(router.release(code));
            }
        }
    }

    #[test]
    fn cluster_modifiers_and_repeat_policy() {
        let mut router = KeyRouter::default();
        router.set_mode(NavigationMode::Quadtree);
        router.activate();
        for (code, quadrant) in [
            (4, Quadrant::TopLeft),
            (37, Quadrant::TopRight),
            (38, Quadrant::BottomLeft),
            (40, Quadrant::BottomRight),
        ] {
            assert_eq!(
                router.press(code, Modifiers::default(), false),
                (true, Some(Command::Descend(quadrant)))
            );
            assert_eq!(
                router.press(code, Modifiers::default(), true),
                (true, Some(Command::Descend(quadrant)))
            );
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
                assert_eq!(
                    router.press(code, modifiers, true),
                    (true, if shift { None } else { Some(Command::Back) })
                );
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
            (true, Some(Command::Dismiss))
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
            (true, Some(Command::Dismiss))
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
                    (true, Some(Command::Dismiss))
                );
                assert!(!router.is_active());
                assert!(router.release(code));
            }
        }
    }
    #[test]
    fn mode_switch_routes_immediately_and_preserves_key_ownership() {
        let mut router = KeyRouter::default();
        router.set_mode(NavigationMode::Quadtree);
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
        ] {
            assert_eq!(
                router.press(code, Modifiers::default(), false),
                (true, Some(command))
            );
            assert_eq!(
                router.press(code, Modifiers::default(), true),
                (true, Some(command))
            );
            assert!(router.release(code));
        }
        let shifted = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        for (code, refine) in [(38, false), (40, true)] {
            let command = Command::ResizeGrid { refine };
            assert_eq!(router.press(code, shifted, false), (true, Some(command)));
            assert_eq!(router.press(code, shifted, true), (true, Some(command)));
            // Releasing Shift cannot turn a held density key into movement.
            assert_eq!(router.press(code, Modifiers::default(), true), (true, None));
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
            router.press(32, Modifiers::default(), false),
            (true, Some(Command::Dismiss))
        );
        assert!(router.release(32));
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
            (true, Some(Command::PreviousDisplay))
        );
        assert!(router.release(4));
        router.deactivate();
        router.set_mode(NavigationMode::Grid);
        router.activate();
        assert_eq!(router.mode(), NavigationMode::Grid);
        router.deactivate();
        router.activate();
        assert_eq!(router.mode(), NavigationMode::Grid);
        assert_eq!(
            router.press(4, Modifiers::default(), false),
            (true, Some(Command::Move(Direction::Left)))
        );
        assert!(router.release(4));
    }

    #[test]
    fn held_navigation_cannot_cross_modes_modifiers_or_sessions() {
        let mut router = KeyRouter::default();
        router.set_mode(NavigationMode::Quadtree);
        let plain = Modifiers::default();
        let shifted = Modifiers {
            shift: true,
            ..plain
        };
        router.activate();
        assert_eq!(
            router.press(38, plain, false),
            (true, Some(Command::Descend(Quadrant::BottomLeft)))
        );
        assert!(router.press(38, plain, true).1.is_some());
        assert_eq!(
            router.press(46, plain, false),
            (true, Some(Command::SwitchMode(NavigationMode::Grid)))
        );
        assert_eq!(router.press(38, plain, true), (true, None));
        assert!(router.release(38));
        assert!(router.release(46));
        assert_eq!(
            router.press(38, plain, false),
            (true, Some(Command::Move(Direction::Down)))
        );
        assert_eq!(
            router.press(38, plain, true),
            (true, Some(Command::Move(Direction::Down)))
        );
        assert_eq!(router.press(38, shifted, true), (true, None));
        assert_eq!(router.press(38, plain, true), (true, None));
        assert!(router.release(38));
        assert_eq!(
            router.press(4, plain, false),
            (true, Some(Command::Move(Direction::Left)))
        );
        assert_eq!(router.modifier_changed(56, true), None);
        assert_eq!(router.modifier_changed(56, false), None);
        assert_eq!(router.press(4, plain, true), (true, None));
        assert!(router.release(4));

        for code in [4, 37] {
            assert!(router.press(code, plain, false).1.is_some());
            router.deactivate();
            assert_eq!(router.press(code, plain, true), (true, None));
            router.set_mode(NavigationMode::Quadtree);
            router.activate();
            assert_eq!(router.press(code, plain, true), (true, None));
            assert!(router.release(code));
            assert_eq!(
                router.press(46, plain, false),
                (true, Some(Command::SwitchMode(NavigationMode::Grid)))
            );
            assert!(router.release(46));
        }
        // A key held in another app never becomes ours merely by activating.
        router.deactivate();
        assert_eq!(router.press(40, plain, false), (false, None));
        router.activate();
        assert_eq!(
            router.press(40, plain, true),
            (false, Some(Command::Dismiss))
        );
        assert!(!router.release(40));
        router.activate();
        assert!(router.press(32, plain, false).0);
        router.interception_lost();
        router.activate();
        assert_eq!(
            router.press(32, plain, true),
            (false, Some(Command::Dismiss))
        );
        assert!(router.is_drained());
    }

    #[test]
    fn click_reset_and_display_switches_never_repeat_in_either_mode() {
        let mut router = KeyRouter::default();
        router.set_mode(NavigationMode::Quadtree);
        let plain = Modifiers::default();
        let shifted = Modifiers {
            shift: true,
            ..plain
        };
        router.activate();
        for mode in [NavigationMode::Quadtree, NavigationMode::Grid] {
            assert_eq!(router.mode, mode);
            for (code, modifiers, command) in [
                (36, plain, Command::Click),
                (76, plain, Command::Click),
                (31, shifted, Command::Reset),
                (51, shifted, Command::Reset),
                (4, shifted, Command::PreviousDisplay),
                (37, shifted, Command::NextDisplay),
            ] {
                assert_eq!(router.press(code, modifiers, false), (true, Some(command)));
                assert_eq!(router.press(code, modifiers, true), (true, None));
                assert_eq!(router.press(code, plain, true), (true, None));
                assert!(router.release(code));
            }
            assert!(router.press(46, plain, false).0);
            assert_eq!(router.press(46, plain, true), (true, None));
            assert!(router.release(46));
        }
    }

    #[test]
    fn escape_and_changed_modifiers_keep_paired_releases_owned() {
        let mut router = KeyRouter::default();
        router.set_mode(NavigationMode::Quadtree);
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
        router.set_mode(NavigationMode::Quadtree);
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
                (true, Some(Command::Dismiss))
            );
            assert!(!router.is_active());
            assert!(router.release(49));
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
    fn dismissal_keys_are_silent_and_preserve_existing_target_presses() {
        let mut router = KeyRouter::default();
        router.set_mode(NavigationMode::Quadtree);
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
            (true, Some(Command::Dismiss))
        );
        assert!(!router.is_active());
        assert_eq!(router.press(0, Modifiers::default(), true), (true, None));
        router.activate();
        assert_eq!(router.press(0, Modifiers::default(), true), (true, None));
        assert!(router.release(0));
        router.deactivate();
        assert_eq!(router.press(0, Modifiers::default(), false), (false, None));
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
                (true, Some(Command::Dismiss))
            );
            assert!(router.release(code));
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
            (true, Some(Command::Dismiss))
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
