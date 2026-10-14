//! Physical key ownership ensures an intercepted press never leaks its release.
use crate::navigation::Quadrant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Descend(Quadrant),
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
}

pub struct KeyRouter {
    active: bool,
    held: [bool; 256],
}

impl Default for KeyRouter {
    fn default() -> Self {
        Self {
            active: false,
            held: [false; 256],
        }
    }
}

impl KeyRouter {
    pub fn activate(&mut self) {
        self.active = true;
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
        // Shift belongs to navigation (notably Shift+Backspace). Modifier events
        // themselves always pass through so the target keeps its modifier state.
        if !self.active || !pressed || matches!(code, 56 | 60) {
            return None;
        }
        self.active = false;
        Some(Command::Dismiss)
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
        if code == 160 && !modifiers.shortcut && !modifiers.shift && !repeat {
            *held = true;
            return (true, Some(Command::Activate));
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
        let command = if code == 53 {
            Some(Command::Exit)
        } else if modifiers.shortcut {
            None
        } else {
            match code {
                32 => Some(Command::Descend(Quadrant::TopLeft)),
                34 => Some(Command::Descend(Quadrant::TopRight)),
                38 => Some(Command::Descend(Quadrant::BottomLeft)),
                40 => Some(Command::Descend(Quadrant::BottomRight)),
                51 if modifiers.shift => Some(Command::Reset),
                51 => Some(Command::Back),
                36 | 76 if !modifiers.shift => Some(Command::Click),
                _ => None,
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
        assert_eq!(
            router.press(
                36,
                Modifiers {
                    shift: true,
                    shortcut: false
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
                    shortcut: true
                },
                false
            ),
            (false, Some(Command::Dismiss))
        );
        assert!(!router.is_active());
        router.activate();
        assert_eq!(
            router.press(
                51,
                Modifiers {
                    shift: true,
                    shortcut: false
                },
                false
            ),
            (true, Some(Command::Reset))
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
                    shortcut: true
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
                    shortcut: true
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
    fn mission_control_activates_idle_and_owns_repeat_and_release() {
        let mut router = KeyRouter::default();
        assert_eq!(
            router.press(160, Modifiers::default(), false),
            (true, Some(Command::Activate))
        );
        assert_eq!(router.press(160, Modifiers::default(), true), (true, None));
        assert!(router.release(160));
        assert!(
            !router
                .press(
                    160,
                    Modifiers {
                        shift: false,
                        shortcut: true
                    },
                    false
                )
                .0
        );
        assert!(!router.press(99, Modifiers::default(), false).0);
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

        for code in [56, 60] {
            router.activate();
            assert_eq!(router.modifier_changed(code, true), None);
            assert_eq!(router.modifier_changed(code, false), None);
            assert!(router.is_active());
        }
        for code in [54, 55, 58, 61, 59, 62, 63, 57] {
            router.activate();
            assert_eq!(router.modifier_changed(code, false), None);
            assert!(router.is_active());
            assert_eq!(router.modifier_changed(code, true), Some(Command::Dismiss));
            assert!(!router.is_active());
        }
    }
}
