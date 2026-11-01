//! Native Windows support for the ten-button RedOctane USB soft mat.
//! It reports buttons, not keyboard keys, and identifies itself as 054c:0268.

#[cfg(windows)]
pub use windows::DanceMat;

#[cfg(not(windows))]
#[derive(Default)]
pub struct DanceMat {}

#[cfg(not(windows))]
impl DanceMat {
    pub fn poll(&mut self, _enabled: bool) -> Vec<&'static str> {
        Vec::new()
    }
}

// Button order verified against the connected RedOctane mat. The other six
// panels use ordinary picture keys, so they participate in the selected pack.
#[cfg(any(windows, test))]
const BUTTON_KEYS: [&str; 10] = [
    "Left", "Down", "Up", "Right", "F1", "F2", "F3", "F4", "Escape", "Space",
];

#[cfg(any(windows, test))]
#[derive(Default)]
struct PressState(u32);

#[cfg(any(windows, test))]
impl PressState {
    fn update(&mut self, buttons: u32, enabled: bool) -> Vec<&'static str> {
        let pressed = buttons & !self.0;
        // Track releases even while settings or another app has focus. Returning
        // to the game must not replay steps made while it was inactive.
        self.0 = buttons;
        BUTTON_KEYS
            .iter()
            .enumerate()
            .filter_map(|(index, key)| (enabled && pressed & (1 << index) != 0).then_some(*key))
            .collect()
    }
}

#[cfg(windows)]
mod windows {
    use std::time::{Duration, Instant};

    use windows_sys::Win32::{
        Media::Multimedia::{
            JOY_RETURNBUTTONS, JOYCAPSW, JOYERR_NOERROR, JOYINFOEX, joyGetDevCapsW, joyGetNumDevs,
            joyGetPosEx,
        },
        UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    };

    use super::PressState;

    struct Device {
        id: u32,
        presses: PressState,
    }

    #[derive(Default)]
    pub struct DanceMat {
        devices: Vec<Device>,
        last_scan: Option<Instant>,
    }

    impl DanceMat {
        pub fn poll(&mut self, enabled: bool) -> Vec<&'static str> {
            let now = Instant::now();
            if self
                .last_scan
                .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(1))
            {
                self.scan();
                self.last_scan = Some(now);
            }

            let enabled = enabled && game_is_foreground();
            let mut keys = Vec::new();
            self.devices.retain_mut(|device| {
                let Some(buttons) = read_buttons(device.id) else {
                    return false;
                };
                keys.extend(device.presses.update(buttons, enabled));
                true
            });
            keys
        }

        fn scan(&mut self) {
            // SAFETY: This function has no arguments or memory requirements.
            for id in 0..unsafe { joyGetNumDevs() } {
                if self.devices.iter().any(|device| device.id == id) {
                    continue;
                }
                let mut caps = JOYCAPSW::default();
                // SAFETY: caps is writable and its exact size is passed to WinMM.
                let result =
                    unsafe { joyGetDevCapsW(id as usize, &mut caps, size_of::<JOYCAPSW>() as u32) };
                // Some soft mats reuse Sony's PS3 ID. Restrict the profile to
                // the ten-button, axis-free device so real PS3 pads aren't mapped
                // as a RedOctane mat.
                if result == JOYERR_NOERROR
                    && caps.wMid == 0x054c
                    && caps.wPid == 0x0268
                    && caps.wNumButtons == 10
                    && caps.wNumAxes == 0
                    && let Some(buttons) = read_buttons(id)
                {
                    self.devices.push(Device {
                        id,
                        presses: PressState(buttons),
                    });
                }
            }
        }
    }

    fn read_buttons(id: u32) -> Option<u32> {
        let mut state = JOYINFOEX {
            dwSize: size_of::<JOYINFOEX>() as u32,
            dwFlags: JOY_RETURNBUTTONS as u32,
            ..Default::default()
        };
        // SAFETY: state is initialized with the required size and flags, and
        // remains valid and writable for the duration of the call.
        (unsafe { joyGetPosEx(id, &mut state) } == JOYERR_NOERROR).then_some(state.dwButtons)
    }

    fn game_is_foreground() -> bool {
        let mut process_id = 0;
        // SAFETY: The OS owns the foreground handle; process_id is writable.
        unsafe { GetWindowThreadProcessId(GetForegroundWindow(), &mut process_id) };
        process_id == std::process::id()
    }
}

#[cfg(test)]
mod tests {
    use super::PressState;

    #[test]
    fn captured_redoctane_buttons_match_directions() {
        for (buttons, expected) in [(4, "Up"), (2, "Down"), (1, "Left"), (8, "Right")] {
            assert_eq!(PressState::default().update(buttons, true), [expected]);
        }
    }

    #[test]
    fn steps_fire_once_until_released_and_allow_opposite_panels() {
        let mut state = PressState::default();
        assert_eq!(state.update(3, true), ["Left", "Down"]);
        assert!(state.update(3, true).is_empty());
        assert_eq!(state.update(15, true), ["Up", "Right"]);
        assert!(state.update(0, true).is_empty());
        assert_eq!(state.update(15, true), ["Left", "Down", "Up", "Right"]);
    }

    #[test]
    fn inactive_steps_are_not_replayed_and_extra_panels_work() {
        let mut state = PressState::default();
        assert!(state.update(4, false).is_empty());
        assert!(state.update(4, true).is_empty());
        assert!(state.update(0, false).is_empty());
        assert_eq!(state.update(4, true), ["Up"]);
        assert_eq!(
            state.update(0x3f0, true),
            ["F1", "F2", "F3", "F4", "Escape", "Space"]
        );
    }
}
