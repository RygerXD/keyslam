use std::{
    env, io,
    path::{Path, PathBuf},
};

#[cfg(not(target_os = "windows"))]
use std::process::Command;
#[cfg(target_os = "windows")]
use std::{os::windows::ffi::OsStrExt, ptr};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};

pub fn executable_directory() -> io::Result<PathBuf> {
    let executable = env::current_exe()?;
    executable.parent().map(Path::to_path_buf).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "the KeySlam executable has no parent folder",
        )
    })
}

pub fn open_directory(path: &Path) -> io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        let operation = "explore\0".encode_utf16().collect::<Vec<_>>();
        let path = path
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        // SAFETY: Both strings are null-terminated and live for the duration of the call.
        let result = unsafe {
            ShellExecuteW(
                ptr::null_mut(),
                operation.as_ptr(),
                path.as_ptr(),
                ptr::null(),
                ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        if result as isize > 32 {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "Windows could not open the folder (shell error {})",
                result as isize
            )))
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        #[cfg(target_os = "macos")]
        let mut command = {
            let mut command = Command::new("open");
            command.arg(path);
            command
        };
        #[cfg(all(unix, not(target_os = "macos")))]
        let mut command = {
            let mut command = Command::new("xdg-open");
            command.arg(path);
            command
        };
        #[cfg(not(any(target_os = "macos", unix)))]
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "opening folders is not supported on this platform",
        ));

        command.spawn().map(|_| ())
    }
}
