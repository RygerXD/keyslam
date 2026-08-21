use crossbeam_channel::Sender;

#[derive(Debug)]
pub enum PlatformEvent {
    Key(String),
}

#[cfg(windows)]
mod keyboard {
    use std::{
        io::{BufRead, BufReader, Read, Write},
        os::windows::process::CommandExt,
        process::{Child, ChildStdin, Command, Stdio},
        ptr,
        sync::{
            OnceLock,
            atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering},
        },
        thread::{self, JoinHandle},
    };

    use crossbeam_channel::{Sender, bounded};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE, LPARAM, LRESULT, WAIT_OBJECT_0, WPARAM},
        System::Threading::{
            CreateEventW, EVENT_MODIFY_STATE, GetCurrentThreadId, OpenEventW, SetEvent,
            WaitForSingleObject,
        },
        UI::{
            Input::KeyboardAndMouse::{
                GetAsyncKeyState, VK_BACK, VK_CLEAR, VK_CONTROL, VK_DOWN, VK_END, VK_ESCAPE, VK_F4,
                VK_HELP, VK_HOME, VK_INSERT, VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN,
                VK_MENU, VK_NEXT, VK_NUMLOCK, VK_PAUSE, VK_PRIOR, VK_RCONTROL, VK_RIGHT, VK_RMENU,
                VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_SNAPSHOT, VK_SPACE, VK_TAB, VK_UP,
            },
            WindowsAndMessaging::{
                CallNextHookEx, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId,
                KBDLLHOOKSTRUCT, MSG, PM_NOREMOVE, PeekMessageW, PostThreadMessageW,
                SetWindowsHookExW, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP,
                WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
            },
        },
    };

    use super::PlatformEvent;

    static EVENTS: OnceLock<Sender<PlatformEvent>> = OnceLock::new();
    static PARENT_PROCESS_ID: AtomicU32 = AtomicU32::new(0);
    static PARENT_EXIT_EVENT: AtomicUsize = AtomicUsize::new(0);
    static HELPER_EXIT_EVENT: AtomicUsize = AtomicUsize::new(0);
    static EXIT_REQUESTED: AtomicBool = AtomicBool::new(false);
    static ALT_DOWN: AtomicBool = AtomicBool::new(false);
    static SHIFT_DOWN: AtomicBool = AtomicBool::new(false);
    static STAR_DOWN: AtomicBool = AtomicBool::new(false);
    static WINDOWS_KEYS_DOWN: AtomicU32 = AtomicU32::new(0);
    static NUMPAD_DOWN: [AtomicU64; 4] = [
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
        AtomicU64::new(0),
    ];
    const LLKHF_EXTENDED: u32 = 0x01;
    pub(super) const LLKHF_ALTDOWN: u32 = 0x20;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    pub const HELPER_ARGUMENT_PREFIX: &str = "--keyboard-guard-helper=";

    pub struct KeyboardGuard {
        child: Child,
        stdin: Option<ChildStdin>,
        reader: Option<JoinHandle<()>>,
        exit_event: HANDLE,
    }

    impl KeyboardGuard {
        pub fn install(sender: Sender<PlatformEvent>) -> Result<Self, String> {
            // The full eframe process successfully registered WH_KEYBOARD_LL
            // but USER32 never invoked its callback. Run the same hook in a
            // minimal hidden mode of this executable and keep it tied to the
            // game through pipes instead.
            let executable = std::env::current_exe().map_err(|error| {
                format!("Windows keyboard protection could not locate KeySlam: {error}")
            })?;
            let process_id = std::process::id();
            let event_name = wide(&exit_event_name(process_id));
            let exit_event = unsafe { CreateEventW(ptr::null(), 0, 0, event_name.as_ptr()) };
            if exit_event.is_null() {
                return Err(format!(
                    "Windows keyboard exit authorization could not start: {}",
                    std::io::Error::last_os_error()
                ));
            }
            PARENT_EXIT_EVENT.store(exit_event as usize, Ordering::SeqCst);

            let mut child = match Command::new(executable)
                .arg(format!("{HELPER_ARGUMENT_PREFIX}{process_id}"))
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
            {
                Ok(child) => child,
                Err(error) => {
                    close_exit_event(exit_event);
                    return Err(format!(
                        "Windows keyboard protection could not start: {error}"
                    ));
                }
            };
            let stdin = child.stdin.take().ok_or_else(|| {
                stop_child(&mut child);
                close_exit_event(exit_event);
                "Windows keyboard protection could not open its lifetime pipe".to_owned()
            })?;
            let stdout = child.stdout.take().ok_or_else(|| {
                stop_child(&mut child);
                close_exit_event(exit_event);
                "Windows keyboard protection could not open its event pipe".to_owned()
            })?;
            let mut output = BufReader::new(stdout);
            let mut readiness = String::new();
            if let Err(error) = output.read_line(&mut readiness) {
                stop_child(&mut child);
                close_exit_event(exit_event);
                return Err(format!(
                    "Windows keyboard protection stopped during startup: {error}"
                ));
            }
            if readiness.trim_end() != "READY" {
                stop_child(&mut child);
                close_exit_event(exit_event);
                let detail = readiness
                    .trim_end()
                    .strip_prefix("ERROR\t")
                    .unwrap_or("helper did not report ready");
                return Err(format!(
                    "Windows keyboard protection could not start: {detail}"
                ));
            }

            let reader = thread::Builder::new()
                .name("windows-keyboard-events".to_owned())
                .spawn(move || forward_helper_events(output, sender))
                .map_err(|error| {
                    stop_child(&mut child);
                    close_exit_event(exit_event);
                    format!("Windows keyboard event reader could not start: {error}")
                })?;
            Ok(Self {
                child,
                stdin: Some(stdin),
                reader: Some(reader),
                exit_event,
            })
        }
    }

    impl Drop for KeyboardGuard {
        fn drop(&mut self) {
            self.stdin.take();
            stop_child(&mut self.child);
            if let Some(reader) = self.reader.take() {
                let _ = reader.join();
            }
            close_exit_event(self.exit_event);
        }
    }

    fn stop_child(child: &mut Child) {
        let _ = child.kill();
        let _ = child.wait();
    }

    fn close_exit_event(event: HANDLE) {
        if !event.is_null() {
            PARENT_EXIT_EVENT.store(0, Ordering::SeqCst);
            unsafe {
                CloseHandle(event);
            }
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain([0]).collect()
    }

    fn exit_event_name(parent_process_id: u32) -> String {
        format!("Local\\KeySlamExit-{parent_process_id}")
    }

    fn forward_helper_events(
        mut output: BufReader<std::process::ChildStdout>,
        sender: Sender<PlatformEvent>,
    ) {
        let mut line = String::new();
        loop {
            line.clear();
            match output.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let event = line.trim_end();
                    if let Some(key) = event.strip_prefix("KEY\t") {
                        let _ = sender.try_send(PlatformEvent::Key(key.to_owned()));
                    }
                }
            }
        }
    }

    pub fn run_helper(parent_process_id: u32) -> Result<(), String> {
        PARENT_PROCESS_ID.store(parent_process_id, Ordering::SeqCst);
        let event_name = wide(&exit_event_name(parent_process_id));
        let exit_event = unsafe { OpenEventW(EVENT_MODIFY_STATE, 0, event_name.as_ptr()) };
        if exit_event.is_null() {
            let error = std::io::Error::last_os_error().to_string();
            println!("ERROR\t{error}");
            let _ = std::io::stdout().flush();
            return Err(error);
        }
        HELPER_EXIT_EVENT.store(exit_event as usize, Ordering::SeqCst);
        let (event_sender, event_receiver) = bounded(128);
        EVENTS
            .set(event_sender)
            .map_err(|_| "keyboard helper was initialized twice".to_owned())?;

        let thread_id = unsafe { GetCurrentThreadId() };
        let mut message = MSG::default();
        unsafe {
            PeekMessageW(&mut message, ptr::null_mut(), 0, 0, PM_NOREMOVE);
        }
        let hook =
            unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_callback), ptr::null_mut(), 0) };
        if hook.is_null() {
            let error = std::io::Error::last_os_error().to_string();
            println!("ERROR\t{error}");
            let _ = std::io::stdout().flush();
            unsafe {
                CloseHandle(exit_event);
            }
            return Err(error);
        }

        thread::Builder::new()
            .name("keyboard-helper-parent-watch".to_owned())
            .spawn(move || {
                let mut byte = [0_u8; 1];
                let _ = std::io::stdin().read(&mut byte);
                unsafe {
                    PostThreadMessageW(thread_id, WM_QUIT, 0, 0);
                }
            })
            .map_err(|error| format!("keyboard helper parent watch failed: {error}"))?;
        thread::Builder::new()
            .name("keyboard-helper-events".to_owned())
            .spawn(move || write_helper_events(event_receiver))
            .map_err(|error| format!("keyboard helper event writer failed: {error}"))?;
        println!("READY");
        std::io::stdout()
            .flush()
            .map_err(|error| format!("keyboard helper readiness failed: {error}"))?;

        while unsafe { GetMessageW(&mut message, ptr::null_mut(), 0, 0) } > 0 {}
        unsafe {
            UnhookWindowsHookEx(hook);
            CloseHandle(exit_event);
        }
        Ok(())
    }

    fn write_helper_events(receiver: crossbeam_channel::Receiver<PlatformEvent>) {
        let stdout = std::io::stdout();
        while let Ok(event) = receiver.recv() {
            let mut output = stdout.lock();
            let result = match event {
                PlatformEvent::Key(key) => writeln!(output, "KEY\t{key}"),
            };
            if result.is_err() || output.flush().is_err() {
                break;
            }
        }
    }

    unsafe extern "system" fn hook_callback(code: i32, message: WPARAM, data: LPARAM) -> LRESULT {
        if code >= 0 {
            let keyboard = unsafe { &*(data as *const KBDLLHOOKSTRUCT) };
            let virtual_key = keyboard.vkCode;
            let is_down = message == WM_KEYDOWN as usize || message == WM_SYSKEYDOWN as usize;
            let is_up = message == WM_KEYUP as usize || message == WM_SYSKEYUP as usize;

            if is_up
                && let Some(bit) = windows_key_bit(virtual_key)
                && WINDOWS_KEYS_DOWN.fetch_and(!bit, Ordering::SeqCst) & bit != 0
            {
                if let Some(key_name) = blocked_key_name(virtual_key)
                    && let Some(events) = EVENTS.get()
                {
                    let _ = events.try_send(PlatformEvent::Key(key_name.to_owned()));
                }
                return 1;
            }
            if !parent_is_foreground() {
                return unsafe { CallNextHookEx(ptr::null_mut(), code, message, data) };
            }
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
                if is_down && !EXIT_REQUESTED.swap(true, Ordering::SeqCst) {
                    let exit_event = HELPER_EXIT_EVENT.load(Ordering::SeqCst) as HANDLE;
                    if !exit_event.is_null() {
                        unsafe {
                            SetEvent(exit_event);
                        }
                    }
                } else if is_up {
                    EXIT_REQUESTED.store(false, Ordering::SeqCst);
                }
                // Preserve the real Alt+F4 so its native close message wakes
                // eframe. The parent consumes the event above before applying
                // its kiosk close guard, including after window recreation.
                return unsafe { CallNextHookEx(ptr::null_mut(), code, message, data) };
            }

            // The Start menu opens on release if either half of the Windows
            // logo key reaches Explorer, so consume both transitions before
            // any gameplay event forwarding.
            if virtual_key == VK_LWIN as u32 || virtual_key == VK_RWIN as u32 {
                if is_down && let Some(bit) = windows_key_bit(virtual_key) {
                    WINDOWS_KEYS_DOWN.fetch_or(bit, Ordering::SeqCst);
                }
                return 1;
            }

            // Num Lock is not represented by egui's Key enum, so forward it
            // through the native hook on release like the other special keys,
            // while preserving its normal lock toggle.
            if virtual_key == VK_NUMLOCK as u32 {
                if is_up && let Some(events) = EVENTS.get() {
                    let _ = events.try_send(PlatformEvent::Key("NumLock".to_owned()));
                }
                return unsafe { CallNextHookEx(ptr::null_mut(), code, message, data) };
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

    fn parent_is_foreground() -> bool {
        let foreground = unsafe { GetForegroundWindow() };
        let mut process_id = 0;
        unsafe {
            GetWindowThreadProcessId(foreground, &mut process_id);
        }
        process_id == PARENT_PROCESS_ID.load(Ordering::SeqCst)
    }

    fn windows_key_bit(virtual_key: u32) -> Option<u32> {
        match virtual_key {
            value if value == VK_LWIN as u32 => Some(1),
            value if value == VK_RWIN as u32 => Some(2),
            _ => None,
        }
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
        let exit_event = PARENT_EXIT_EVENT.load(Ordering::SeqCst) as HANDLE;
        !exit_event.is_null() && unsafe { WaitForSingleObject(exit_event, 0) } == WAIT_OBJECT_0
    }

    pub fn exit_chord_down() -> bool {
        unsafe { GetAsyncKeyState(VK_MENU as i32) < 0 && GetAsyncKeyState(VK_F4 as i32) < 0 }
    }

    pub fn modifier_key_is_down(key_name: &str) -> bool {
        let virtual_key = match key_name {
            "AltLeft" => VK_LMENU,
            "AltRight" => VK_RMENU,
            _ => return true,
        };
        unsafe { GetAsyncKeyState(virtual_key as i32) < 0 }
    }
}

#[cfg(not(windows))]
mod keyboard {
    use crossbeam_channel::Sender;

    use super::PlatformEvent;

    pub const HELPER_ARGUMENT_PREFIX: &str = "--keyboard-guard-helper=";

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

    pub fn modifier_key_is_down(_key_name: &str) -> bool {
        true
    }

    pub fn run_helper(_parent_process_id: u32) -> Result<(), String> {
        Err("Windows keyboard helper is only available on Windows".to_owned())
    }
}

pub use keyboard::{
    HELPER_ARGUMENT_PREFIX, KeyboardGuard, exit_chord_down, modifier_key_is_down,
    pressed_numpad_key, run_helper as run_keyboard_guard_helper, take_exit_requested,
};

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
    // lift an auto-hidden taskbar over KeySlam.
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
