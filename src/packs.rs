use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};

use serde::Deserialize;

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
    crate::assets::path("packs")
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
