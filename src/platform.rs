use crossbeam_channel::Sender;

#[derive(Debug)]
pub enum PlatformEvent {
    Key(String),
}

#[cfg(windows)]
mod keyboard {
    use std::{
        ptr,
        sync::{
            OnceLock,
            atomic::{AtomicBool, AtomicU64, Ordering},
        },
        thread::{self, JoinHandle},
    };

    use crossbeam_channel::Sender;
    use windows_sys::Win32::{
        Foundation::{LPARAM, LRESULT, WPARAM},
        System::{LibraryLoader::GetModuleHandleW, Threading::GetCurrentThreadId},
        UI::{
            Input::KeyboardAndMouse::{
                GetAsyncKeyState, VK_BACK, VK_CLEAR, VK_CONTROL, VK_DOWN, VK_END, VK_ESCAPE, VK_F4,
                VK_HELP, VK_HOME, VK_INSERT, VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN,
                VK_MENU, VK_NEXT, VK_PAUSE, VK_PRIOR, VK_RCONTROL, VK_RIGHT, VK_RMENU, VK_RSHIFT,
                VK_RWIN, VK_SHIFT, VK_SNAPSHOT, VK_SPACE, VK_TAB, VK_UP,
            },
            WindowsAndMessaging::{
                CallNextHookEx, GetMessageW, KBDLLHOOKSTRUCT, MSG, PM_NOREMOVE, PeekMessageW,
                PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx, WH_KEYBOARD_LL,
                WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
            },
        },
    };

    use super::PlatformEvent;

    static EVENTS: OnceLock<Sender<PlatformEvent>> = OnceLock::new();
    static EXIT_REQUESTED: AtomicBool = AtomicBool::new(false);
    static ALT_DOWN: AtomicBool = AtomicBool::new(false);
    static SHIFT_DOWN: AtomicBool = AtomicBool::new(false);
    static STAR_DOWN: AtomicBool = AtomicBool::new(false);
    static NUMPAD_DOWN: [AtomicU64; 4] = [
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
    ];
    const LLKHF_EXTENDED: u32 = 0x01;
    pub(super) const LLKHF_ALTDOWN: u32 = 0x20;

    pub struct KeyboardGuard {
        thread_id: u32,
        thread: Option<JoinHandle<()>>,
    }

    impl KeyboardGuard {
        pub fn install(sender: Sender<PlatformEvent>) -> Result<Self, String> {
            let _ = EVENTS.set(sender);
            let (ready_sender, ready_receiver) = std::sync::mpsc::sync_channel(1);
            let thread = thread::Builder::new()
                .name("windows-keyboard-guard".to_owned())
                .spawn(move || run_hook(ready_sender))
                .map_err(|error| format!("Windows keyboard protection could not start: {error}"))?;
            match ready_receiver.recv() {
                Ok(Ok(thread_id)) => Ok(Self {
                    thread_id,
                    thread: Some(thread),
                }),
                Ok(Err(error)) => {
                    let _ = thread.join();
                    Err(error)
                }
                Err(_) => {
                    let _ = thread.join();
                    Err("Windows keyboard protection stopped during startup".to_owned())
                }
            }
        }
    }

    impl Drop for KeyboardGuard {
        fn drop(&mut self) {
            unsafe {
                PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0);
            }
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    fn run_hook(ready: std::sync::mpsc::SyncSender<Result<u32, String>>) {
        let thread_id = unsafe { GetCurrentThreadId() };
        let mut message = MSG::default();
        unsafe {
            // A thread message queue is created lazily. Create it before the
            // guard is reported ready so Drop can always post WM_QUIT.
            PeekMessageW(&mut message, ptr::null_mut(), 0, 0, PM_NOREMOVE);
        }
        let module = unsafe { GetModuleHandleW(ptr::null()) };
        let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_callback), module, 0) };
        if hook.is_null() {
            let _ = ready.send(Err(format!(
                "Windows keyboard protection could not start: {}",
                std::io::Error::last_os_error()
            )));
            return;
        }
        if ready.send(Ok(thread_id)).is_err() {
            unsafe {
                UnhookWindowsHookEx(hook);
            }
            return;
        }
        while unsafe { GetMessageW(&mut message, ptr::null_mut(), 0, 0) } > 0 {}
        unsafe {
            UnhookWindowsHookEx(hook);
        }
    }

    unsafe extern "system" fn hook_callback(code: i32, message: WPARAM, data: LPARAM) -> LRESULT {
        if code >= 0 {
            let keyboard = unsafe { &*(data as *const KBDLLHOOKSTRUCT) };
            let virtual_key = keyboard.vkCode;
            let is_down = message == WM_KEYDOWN as usize || message == WM_SYSKEYDOWN as usize;
            let is_up = message == WM_KEYUP as usize || message == WM_SYSKEYUP as usize;
            let is_alt_key = matches!(
                virtual_key,
                value if value == VK_MENU as u32
                    || value == VK_LMENU as u32
                    || value == VK_RMENU as u32
            );
            if is_alt_key {
                if is_down {
                    ALT_DOWN.store(true, Ordering::SeqCst);
                } else if is_up {
                    ALT_DOWN.store(false, Ordering::SeqCst);
                }
            }
            let is_shift_key = matches!(
                virtual_key,
                value if value == VK_SHIFT as u32
                    || value == VK_LSHIFT as u32
                    || value == VK_RSHIFT as u32
            );
            if is_shift_key {
                SHIFT_DOWN.store(is_down, Ordering::SeqCst);
            }
            let alt = alt_is_down(
                message,
                keyboard.flags,
                ALT_DOWN.load(Ordering::SeqCst) || unsafe { GetAsyncKeyState(VK_MENU as i32) } < 0,
            );
            let control = unsafe { GetAsyncKeyState(VK_CONTROL as i32) } < 0;
            let shift = unsafe {
                SHIFT_DOWN.load(Ordering::SeqCst)
                    || GetAsyncKeyState(VK_SHIFT as i32) < 0
                    || GetAsyncKeyState(VK_LSHIFT as i32) < 0
                    || GetAsyncKeyState(VK_RSHIFT as i32) < 0
            };

            if alt && virtual_key == VK_F4 as u32 {
                if is_down {
                    EXIT_REQUESTED.store(true, Ordering::SeqCst);
                }
                // Alt+F4 is the one native close request the kiosk allows.
                // Pass it onward while authorizing the close guard separately.
                return unsafe { CallNextHookEx(ptr::null_mut(), code, message, data) };
            }

            // Suppress both Windows-logo keys directly on key-down and key-up.
            // If either half reaches Explorer, it can open the Start menu even
            // though the app immediately reclaims its fullscreen window.
            if virtual_key == VK_LWIN as u32 || virtual_key == VK_RWIN as u32 {
                return 1;
            }

            if virtual_key == 0x38 && (shift || STAR_DOWN.load(Ordering::SeqCst)) {
                if is_down
                    && !STAR_DOWN.swap(true, Ordering::SeqCst)
                    && let Some(events) = EVENTS.get()
                {
                    let _ = events.try_send(PlatformEvent::Key("*".to_owned()));
                } else if is_up {
                    STAR_DOWN.store(false, Ordering::SeqCst);
                }
                return 1;
            }

            if let Some(key_name) = numpad_key_name(virtual_key, keyboard.scanCode, keyboard.flags)
            {
                if is_down
                    && mark_numpad_down(virtual_key)
                    && let Some(events) = EVENTS.get()
                {
                    let _ = events.try_send(PlatformEvent::Key(key_name.to_owned()));
                }
                if is_up {
                    mark_numpad_up(virtual_key);
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

    fn mark_numpad_down(virtual_key: u32) -> bool {
        let index = (virtual_key / 64) as usize;
        let bit = 1_u64 << (virtual_key % 64);
        NUMPAD_DOWN[index].fetch_or(bit, Ordering::SeqCst) & bit == 0
    }

    fn mark_numpad_up(virtual_key: u32) {
        let index = (virtual_key / 64) as usize;
        let bit = 1_u64 << (virtual_key % 64);
        NUMPAD_DOWN[index].fetch_and(!bit, Ordering::SeqCst);
    }

    pub(super) fn alt_is_down(message: WPARAM, flags: u32, async_alt_down: bool) -> bool {
        flags & LLKHF_ALTDOWN != 0
            || message == WM_SYSKEYDOWN as usize
            || message == WM_SYSKEYUP as usize
            || async_alt_down
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

    pub(super) fn should_block(key: u32, alt: bool, control: bool) -> bool {
        key <= VK_BACK as u32
            || matches!(
                key,
                value if value == VK_PAUSE as u32
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
            || (alt && key == VK_ESCAPE as u32)
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

    pub fn pressed_numpad_key(digit: usize) -> Option<&'static str> {
        const NUMPAD: [&str; 10] = [
            "NumPad0", "NumPad1", "NumPad2", "NumPad3", "NumPad4", "NumPad5", "NumPad6", "NumPad7",
            "NumPad8", "NumPad9",
        ];
        const NUMLOCK_OFF: [u16; 10] = [
            VK_INSERT, VK_END, VK_DOWN, VK_NEXT, VK_LEFT, VK_CLEAR, VK_RIGHT, VK_HOME, VK_UP,
            VK_PRIOR,
        ];
        let name = NUMPAD.get(digit).copied()?;
        let numlock_on_key = 0x60 + digit as i32;
        let numlock_off_key = i32::from(*NUMLOCK_OFF.get(digit)?);
        let is_down = unsafe {
            GetAsyncKeyState(numlock_on_key) < 0 || GetAsyncKeyState(numlock_off_key) < 0
        };
        is_down.then_some(name)
    }

    pub fn take_exit_requested() -> bool {
        EXIT_REQUESTED.swap(false, Ordering::SeqCst)
    }

    pub fn exit_chord_down() -> bool {
        unsafe { GetAsyncKeyState(VK_MENU as i32) < 0 && GetAsyncKeyState(VK_F4 as i32) < 0 }
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

    pub fn pressed_numpad_key(_digit: usize) -> Option<&'static str> {
        None
    }

    pub fn take_exit_requested() -> bool {
        false
    }

    pub fn exit_chord_down() -> bool {
        false
    }
}

pub use keyboard::{KeyboardGuard, exit_chord_down, pressed_numpad_key, take_exit_requested};

pub fn install_keyboard_guard(sender: Sender<PlatformEvent>) -> Result<KeyboardGuard, String> {
    KeyboardGuard::install(sender)
}

#[cfg(windows)]
fn set_taskbar_z_order(insert_after: windows_sys::Win32::Foundation::HWND) {
    use std::ptr;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowExW, FindWindowW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    };

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain([0]).collect()
    }

    unsafe fn move_in_z_order(
        window: windows_sys::Win32::Foundation::HWND,
        insert_after: windows_sys::Win32::Foundation::HWND,
    ) {
        if !window.is_null() {
            unsafe {
                SetWindowPos(
                    window,
                    insert_after,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
    }

    unsafe {
        let primary_class = wide("Shell_TrayWnd");
        move_in_z_order(
            FindWindowW(primary_class.as_ptr(), ptr::null()),
            insert_after,
        );

        let secondary_class = wide("Shell_SecondaryTrayWnd");
        let mut taskbar = ptr::null_mut();
        loop {
            taskbar = FindWindowExW(
                ptr::null_mut(),
                taskbar,
                secondary_class.as_ptr(),
                ptr::null(),
            );
            if taskbar.is_null() {
                break;
            }
            move_in_z_order(taskbar, insert_after);
        }
    }
}

#[cfg(windows)]
pub fn keep_taskbar_behind() {
    use windows_sys::Win32::UI::WindowsAndMessaging::HWND_BOTTOM;

    // Native fullscreen normally asks Explorer to lower these windows. Keep
    // that z-order asserted as a kiosk fallback so an edge right-click cannot
    // lift an auto-hidden taskbar over BabySmash.
    set_taskbar_z_order(HWND_BOTTOM);
}

#[cfg(not(windows))]
pub fn keep_taskbar_behind() {}

#[cfg(windows)]
pub fn restore_taskbar() {
    use windows_sys::Win32::UI::WindowsAndMessaging::HWND_TOPMOST;

    set_taskbar_z_order(HWND_TOPMOST);
}

#[cfg(not(windows))]
pub fn restore_taskbar() {}

#[cfg(all(test, windows))]
mod tests {
    use super::keyboard::{LLKHF_ALTDOWN, alt_is_down, should_block};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_ESCAPE, VK_MENU, VK_TAB};
    use windows_sys::Win32::UI::WindowsAndMessaging::{WM_KEYDOWN, WM_SYSKEYDOWN};

    #[test]
    fn blocks_windows_app_switching_shortcuts() {
        assert!(should_block(VK_TAB as u32, true, false));
        assert!(should_block(VK_ESCAPE as u32, true, false));
        assert!(should_block(VK_ESCAPE as u32, false, true));
        assert!(!should_block(VK_MENU as u32, false, false));
    }

    #[test]
    fn recognizes_alt_even_when_windows_only_marks_a_system_key() {
        assert!(alt_is_down(WM_SYSKEYDOWN as usize, 0, false));
        assert!(alt_is_down(WM_KEYDOWN as usize, LLKHF_ALTDOWN, false));
        assert!(!alt_is_down(WM_KEYDOWN as usize, 0, false));
    }
}
