use std::{
    fs, io,
    path::{Path, PathBuf},
};

use include_dir::{Dir, include_dir};

#[cfg(not(debug_assertions))]
use crate::paths::executable_directory;

static BUILTIN_ASSETS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets");

pub fn root() -> io::Result<PathBuf> {
    #[cfg(debug_assertions)]
    {
        Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"))
    }
    #[cfg(not(debug_assertions))]
    {
        executable_directory().map(|directory| directory.join("assets"))
    }
}

pub fn path(relative: impl AsRef<Path>) -> io::Result<PathBuf> {
    root().map(|root| root.join(relative))
}

pub fn read(relative: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    fs::read(path(relative)?)
}

pub fn install_builtins() -> Result<(), String> {
    let root = root().map_err(|error| error.to_string())?;
    copy_dir(&BUILTIN_ASSETS, &root)
}

fn copy_dir(directory: &Dir<'_>, root: &Path) -> Result<(), String> {
    for file in directory.files() {
        let destination = root.join(file.path());
        if destination.exists() {
            continue;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::write(destination, file.contents()).map_err(|error| error.to_string())?;
    }
    for child in directory.dirs() {
        copy_dir(child, root)?;
    }
    Ok(())
}
