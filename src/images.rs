use std::{collections::HashMap, fs, path::PathBuf};

use crate::packs;

pub fn next_item_image(
    set: &str,
    item: &str,
    cycles: &mut HashMap<String, usize>,
) -> Option<PathBuf> {
    let directory = packs::item_directory(set, item)?;
    let mut images = fs::read_dir(directory)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
        })
        .collect::<Vec<_>>();
    images.sort_by_key(|path| path.to_string_lossy().to_ascii_lowercase());
    if images.is_empty() {
        return None;
    }

    let key = format!("{set}/{item}").to_ascii_lowercase();
    let cycle = cycles.entry(key).or_default();
    let image = images[*cycle % images.len()].clone();
    *cycle = (*cycle + 1) % images.len();
    Some(image)
}
