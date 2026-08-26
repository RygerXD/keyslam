use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};

use include_dir::{Dir, include_dir};
use serde::Deserialize;

#[cfg(not(debug_assertions))]
use crate::paths::executable_directory;

static BUILTIN_PACKS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/packs");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackItem {
    pub folder: String,
    pub label: String,
    pub fallback_emoji: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PackManifest {
    KeyAssignments { keys: Vec<KeyAssignment> },
    OrderedItems { items: Vec<ManifestItem> },
}

#[derive(Deserialize)]
struct KeyAssignment {
    key: String,
    item: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    emoji: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ManifestItem {
    Folder(String),
    Detailed {
        folder: String,
        #[serde(default)]
        name: String,
        #[serde(default)]
        emoji: String,
    },
}

pub fn root() -> io::Result<PathBuf> {
    #[cfg(debug_assertions)]
    {
        Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/packs"))
    }
    #[cfg(not(debug_assertions))]
    {
        executable_directory().map(|directory| directory.join("packs"))
    }
}

pub fn install_builtin_packs() -> Result<(), String> {
    let root = root().map_err(|error| error.to_string())?;
    copy_dir(&BUILTIN_PACKS, &root)
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

pub fn available() -> Vec<String> {
    let Ok(root) = root() else {
        return Vec::new();
    };
    available_in(&root)
}

fn available_in(root: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut packs = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect::<Vec<_>>();
    packs.sort_by_key(|name| name.to_ascii_lowercase());
    packs
}

pub fn item(pack: &str, key: &str, legacy_index: usize) -> Option<PackItem> {
    item_in(&root().ok()?, pack, key, legacy_index)
}

fn item_in(root: &Path, pack: &str, key: &str, legacy_index: usize) -> Option<PackItem> {
    let directory = pack_directory(root, pack)?;
    match manifest_item(&directory, key, legacy_index) {
        Some(item) => item,
        None => {
            let items = directory_items(&directory);
            items.get(legacy_index % items.len().max(1)).cloned()
        }
    }
}

pub fn item_directory(pack: &str, item: &str) -> Option<PathBuf> {
    let pack = pack_directory(&root().ok()?, pack)?;
    let item = safe_name(item)?;
    let directory = pack.join(item);
    directory.is_dir().then_some(directory)
}

fn pack_directory(root: &Path, pack: &str) -> Option<PathBuf> {
    let pack = safe_name(pack)?;
    let directory = root.join(pack);
    directory.is_dir().then_some(directory)
}

fn safe_name(name: &str) -> Option<&str> {
    let mut components = Path::new(name).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) if !name.is_empty() => Some(name),
        _ => None,
    }
}

fn manifest_item(directory: &Path, key: &str, legacy_index: usize) -> Option<Option<PackItem>> {
    let bytes = fs::read(directory.join("pack.json")).ok()?;
    let manifest = serde_json::from_slice::<PackManifest>(&bytes).ok()?;
    Some(match manifest {
        PackManifest::KeyAssignments { keys } => keys
            .into_iter()
            .find(|assignment| assignment.key.trim().eq_ignore_ascii_case(key))
            .and_then(|assignment| {
                make_item(
                    directory,
                    assignment.item,
                    assignment.name,
                    assignment.emoji,
                )
            }),
        PackManifest::OrderedItems { items } => {
            let items = items
                .into_iter()
                .filter_map(|item| match item {
                    ManifestItem::Folder(folder) => {
                        make_item(directory, folder, String::new(), String::new())
                    }
                    ManifestItem::Detailed {
                        folder,
                        name,
                        emoji,
                    } => make_item(directory, folder, name, emoji),
                })
                .collect::<Vec<_>>();
            items.get(legacy_index % items.len().max(1)).cloned()
        }
    })
}

fn directory_items(directory: &Path) -> Vec<PackItem> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut folders = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect::<Vec<_>>();
    folders.sort_by_key(|name| name.to_ascii_lowercase());
    folders
        .into_iter()
        .filter_map(|folder| make_item(directory, folder, String::new(), String::new()))
        .collect()
}

fn make_item(
    pack_directory: &Path,
    folder: String,
    name: String,
    emoji: String,
) -> Option<PackItem> {
    let folder = safe_name(&folder)?;
    pack_directory.join(folder).is_dir().then(|| PackItem {
        folder: folder.to_owned(),
        label: if name.trim().is_empty() {
            humanize(folder)
        } else {
            name
        },
        fallback_emoji: if emoji.trim().is_empty() {
            "★".to_owned()
        } else {
            emoji
        },
    })
}

fn humanize(folder: &str) -> String {
    let words = folder.replace(['-', '_'], " ");
    let mut characters = words.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_directory(name: &str) -> io::Result<PathBuf> {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        Ok(std::env::temp_dir().join(format!(
            "keyslam-pack-{name}-{}-{nonce}",
            std::process::id()
        )))
    }

    #[test]
    fn names_cannot_escape_the_pack_root() {
        assert!(safe_name("animals").is_some());
        assert!(safe_name("red apple").is_some());
        assert!(safe_name("../sounds").is_none());
        assert!(safe_name("foods/item").is_none());
    }

    #[test]
    fn folder_names_become_readable_labels() {
        assert_eq!(humanize("red-apple"), "Red apple");
        assert_eq!(humanize("musical_score"), "Musical score");
    }

    #[test]
    fn folders_are_dynamic_and_key_assignments_are_authoritative() -> io::Result<()> {
        let root = temporary_directory("dynamic")?;
        let pack = root.join("My new pack");
        fs::create_dir_all(pack.join("alpha_item"))?;
        fs::create_dir_all(pack.join("zebra"))?;
        fs::write(
            pack.join("pack.json"),
            br#"{"keys":[{"key":"Escape","item":"zebra","name":"A Zebra","emoji":"Z"},{"key":"Space","item":"alpha_item"}]}"#,
        )?;

        assert_eq!(available_in(&root), ["My new pack"]);
        assert_eq!(
            item_in(&root, "My new pack", "escape", 0),
            Some(PackItem {
                folder: "zebra".to_owned(),
                label: "A Zebra".to_owned(),
                fallback_emoji: "Z".to_owned(),
            })
        );
        assert_eq!(
            item_in(&root, "My new pack", "Space", 1).map(|item| item.label),
            Some("Alpha item".to_owned())
        );
        assert_eq!(item_in(&root, "My new pack", "F1", 0), None);

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn old_ordered_manifests_still_load() -> io::Result<()> {
        let root = temporary_directory("legacy")?;
        let pack = root.join("Old pack");
        fs::create_dir_all(pack.join("alpha"))?;
        fs::create_dir_all(pack.join("zebra"))?;
        fs::write(pack.join("pack.json"), br#"{"items":["zebra","alpha"]}"#)?;

        assert_eq!(
            item_in(&root, "Old pack", "Space", 1).map(|item| item.folder),
            Some("alpha".to_owned())
        );

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn bundled_manifest_names_the_keys_it_assigns() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/packs");

        assert_eq!(
            item_in(&root, "animals", "Escape", 0).map(|item| item.folder),
            Some("bear".to_owned())
        );
        assert_eq!(
            item_in(&root, "animals", "F1", 2).map(|item| item.folder),
            Some("bee".to_owned())
        );
        assert_eq!(item_in(&root, "animals", "Numpad Multiply", 53), None);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn debug_builds_read_the_editable_source_packs() -> io::Result<()> {
        assert_eq!(
            root()?.canonicalize()?,
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets/packs")
                .canonicalize()?
        );
        Ok(())
    }
}
