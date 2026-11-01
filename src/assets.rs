use std::{
    fs, io,
    path::{Path, PathBuf},
};

use include_dir::{Dir, include_dir};

#[cfg(not(debug_assertions))]
use crate::paths::executable_directory;

static BUILTIN_ASSETS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets");

pub fn root() -> io::Result<PathBuf> {
    if let Some(root) = std::env::var_os("KEYSLAM_ASSETS_DIR") {
        return absolute_root(PathBuf::from(root));
    }
    #[cfg(debug_assertions)]
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    #[cfg(not(debug_assertions))]
    let directory = executable_directory()?;
    match fs::read_to_string(directory.join(".keyslam-assets-path")) {
        Ok(root) => absolute_root(PathBuf::from(root.trim())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(directory.join("assets")),
        Err(error) => Err(error),
    }
}

fn absolute_root(root: PathBuf) -> io::Result<PathBuf> {
    if root.is_absolute() {
        Ok(root)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the personal assets folder must be an absolute path",
        ))
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
        // A personal recording replaces the entire built-in voice cycle for its item.
        if destination
            .extension()
            .is_some_and(|extension| extension == "opus")
            && destination.parent().is_some_and(has_speech)
        {
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

fn has_speech(directory: &Path) -> bool {
    fs::read_dir(directory).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("opus"))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_installation_preserves_personal_voice_cycles()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!("personal-assets-test-{}", std::process::id()));
        let item = root.join("sounds/letters/a");
        fs::create_dir_all(&item)?;
        let personal = item.join("personal.OPUS");
        fs::write(&personal, b"personal recording")?;
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            copy_dir(&BUILTIN_ASSETS, &root)?;
            assert_eq!(fs::read(&personal)?, b"personal recording");
            assert_eq!(fs::read_dir(&item)?.count(), 1);
            assert!(root.join("sounds/letters/b/b.opus").is_file());
            assert!(!root.join("packs/family").exists());
            assert!(absolute_root(PathBuf::from("relative/assets")).is_err());
            Ok(())
        })();
        fs::remove_dir_all(&root)?;
        result
    }
}
