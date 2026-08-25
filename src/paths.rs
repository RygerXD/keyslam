use std::{
    env, io,
    path::{Path, PathBuf},
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
