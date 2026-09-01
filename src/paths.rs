use std::{
    env, io,
    path::{Path, PathBuf},
    process::Command,
};

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
    let mut command = {
        let mut command = Command::new("explorer.exe");
        command.arg(path);
        command
    };
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
    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    return Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "opening folders is not supported on this platform",
    ));

    command.spawn().map(|_| ())
}
