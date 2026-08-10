use crossbeam_channel::Sender;

#[derive(Debug)]
pub enum PlatformEvent {
    Key(String),
    Exit,
}

#[cfg(windows)]
mod keyboard {
    use std::{ptr, sync::OnceLock};

    use crossbeam_channel::Sender;
    use windows_sys::Win32::{
        Foundation::{LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Input::KeyboardAndMouse::{
                GetAsyncKeyState, VK_BACK, VK_CONTROL, VK_ESCAPE, VK_F4, VK_HELP, VK_LCONTROL,
                VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_PAUSE, VK_RCONTROL, VK_RMENU, VK_RSHIFT,
                VK_RWIN, VK_SNAPSHOT, VK_SPACE, VK_TAB,
            },
            WindowsAndMessaging::{
                CallNextHookEx, HHOOK, KBDLLHOOKSTRUCT, SetWindowsHookExW, UnhookWindowsHookEx,
                WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
            },
        },
    };

    use super::PlatformEvent;

    static EVENTS: OnceLock<Sender<PlatformEvent>> = OnceLock::new();
    const LLKHF_EXTENDED: u32 = 0x01;

    pub struct KeyboardGuard {
        hook: HHOOK,
    }

    impl KeyboardGuard {
        pub fn install(sender: Sender<PlatformEvent>) -> Result<Self, String> {
            let _ = EVENTS.set(sender);
            let module = unsafe { GetModuleHandleW(ptr::null()) };
            let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_callback), module, 0) };
            if hook.is_null() {
                Err(format!(
                    "Windows keyboard protection could not start: {}",
                    std::io::Error::last_os_error()
                ))
            } else {
                Ok(Self { hook })
            }
        }
    }

    impl Drop for KeyboardGuard {
        fn drop(&mut self) {
            if !self.hook.is_null() {
                unsafe {
                    UnhookWindowsHookEx(self.hook);
                }
            }
        }
    }

    unsafe extern "system" fn hook_callback(code: i32, message: WPARAM, data: LPARAM) -> LRESULT {
        if code >= 0 {
            let keyboard = unsafe { &*(data as *const KBDLLHOOKSTRUCT) };
            let virtual_key = keyboard.vkCode;
            let is_down = message == WM_KEYDOWN as usize || message == WM_SYSKEYDOWN as usize;
            let is_up = message == WM_KEYUP as usize || message == WM_SYSKEYUP as usize;
            let alt = unsafe { GetAsyncKeyState(VK_MENU as i32) } < 0;
            let control = unsafe { GetAsyncKeyState(VK_CONTROL as i32) } < 0;

            if alt && virtual_key == VK_F4 as u32 {
                if is_down && let Some(events) = EVENTS.get() {
                    let _ = events.try_send(PlatformEvent::Exit);
                }
                return 1;
            }

            if let Some(key_name) = numpad_key_name(virtual_key, keyboard.scanCode, keyboard.flags)
            {
                if is_down && let Some(events) = EVENTS.get() {
                    let _ = events.try_send(PlatformEvent::Key(key_name.to_owned()));
                }
                return 1;
            }

            if should_block(virtual_key, alt, control) {
                if is_up
                    && let Some(key_name) = blocked_key_name(virtual_key)
                    && let Some(events) = EVENTS.get()
                {
                    let _ = events.try_send(PlatformEvent::Key(key_name.to_owned()));
                }
                return 1;
            }
        }
        unsafe { CallNextHookEx(ptr::null_mut(), code, message, data) }
    }

    fn numpad_key_name(virtual_key: u32, scan_code: u32, flags: u32) -> Option<&'static str> {
        const NUMPAD: [&str; 10] = [
            "NumPad0", "NumPad1", "NumPad2", "NumPad3", "NumPad4", "NumPad5", "NumPad6", "NumPad7",
            "NumPad8", "NumPad9",
        ];
        if (0x60..=0x69).contains(&virtual_key) {
            return NUMPAD.get((virtual_key - 0x60) as usize).copied();
        }
        if flags & LLKHF_EXTENDED == 0 {
            return match scan_code {
                0x52 => Some("NumPad0"),
                0x4F => Some("NumPad1"),
                0x50 => Some("NumPad2"),
                0x51 => Some("NumPad3"),
                0x4B => Some("NumPad4"),
                0x4C => Some("NumPad5"),
                0x4D => Some("NumPad6"),
                0x47 => Some("NumPad7"),
                0x48 => Some("NumPad8"),
                0x49 => Some("NumPad9"),
                _ => operator_name(virtual_key),
            };
        }
        operator_name(virtual_key)
    }

    fn operator_name(virtual_key: u32) -> Option<&'static str> {
        match virtual_key {
            0x6A => Some("Multiply"),
            0x6B => Some("Add"),
            0x6D => Some("Subtract"),
            0x6E => Some("Decimal"),
            0x6F => Some("Divide"),
            _ => None,
        }
    }

    fn should_block(key: u32, alt: bool, control: bool) -> bool {
        key <= VK_BACK as u32
            || matches!(
                key,
                value if value == VK_MENU as u32
                    || value == VK_PAUSE as u32
                    || value == VK_SNAPSHOT as u32
                    || value == VK_HELP as u32
                    || value == VK_F4 as u32
            )
            || (0x5B..=0x5F).contains(&key)
            || (0x15..=0x19).contains(&key)
            || (0x1C..=0x1F).contains(&key)
            || (0xA6..=0xAC).contains(&key)
            || (0xB0..=0xB7).contains(&key)
            || (0xE5..=0xFE).contains(&key)
            || (alt && (key == VK_TAB as u32 || key == VK_SPACE as u32))
            || (control && key == VK_ESCAPE as u32)
    }

    fn blocked_key_name(key: u32) -> Option<&'static str> {
        match key {
            value if value == VK_BACK as u32 => Some("Back"),
            value if value == VK_PAUSE as u32 => Some("Pause"),
            value if value == VK_SNAPSHOT as u32 => Some("Snapshot"),
            value if value == VK_F4 as u32 => Some("F4"),
            value if value == VK_LSHIFT as u32 => Some("LeftShift"),
            value if value == VK_RSHIFT as u32 => Some("RightShift"),
            value if value == VK_LCONTROL as u32 => Some("LeftCtrl"),
            value if value == VK_RCONTROL as u32 => Some("RightCtrl"),
            value if value == VK_LMENU as u32 => Some("LeftAlt"),
            value if value == VK_RMENU as u32 => Some("RightAlt"),
            value if value == VK_LWIN as u32 => Some("LWin"),
            value if value == VK_RWIN as u32 => Some("RWin"),
            0x5D => Some("Apps"),
            _ => None,
        }
    }

    pub fn pressed_numpad_key() -> Option<&'static str> {
        const NUMPAD: [&str; 10] = [
            "NumPad0", "NumPad1", "NumPad2", "NumPad3", "NumPad4", "NumPad5", "NumPad6", "NumPad7",
            "NumPad8", "NumPad9",
        ];
        NUMPAD.iter().enumerate().find_map(|(index, name)| {
            let virtual_key = 0x60 + index as i32;
            (unsafe { GetAsyncKeyState(virtual_key) } < 0).then_some(*name)
        })
    }
}

#[cfg(not(windows))]
mod keyboard {
    use crossbeam_channel::Sender;

    use super::PlatformEvent;

    pub struct KeyboardGuard;

    impl KeyboardGuard {
        pub fn install(_sender: Sender<PlatformEvent>) -> Result<Self, String> {
            Ok(Self)
        }
    }

    pub fn pressed_numpad_key() -> Option<&'static str> {
        None
    }
}

pub use keyboard::{KeyboardGuard, pressed_numpad_key};

pub fn install_keyboard_guard(sender: Sender<PlatformEvent>) -> Result<KeyboardGuard, String> {
    KeyboardGuard::install(sender)
}
