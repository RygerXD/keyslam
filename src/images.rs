use std::{collections::HashMap, path::Path};

use include_dir::{Dir, File, include_dir};

static IMAGES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/images");

pub fn all_image_files() -> Vec<&'static File<'static>> {
    let mut files = Vec::new();
    collect_files(&IMAGES, &mut files);
    files
}

fn collect_files(dir: &'static Dir<'static>, files: &mut Vec<&'static File<'static>>) {
    files.extend(dir.files().filter(|file| {
        file.path()
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
    }));
    for child in dir.dirs() {
        collect_files(child, files);
    }
}

pub fn next_animal_image(
    animal: &str,
    cycles: &mut HashMap<String, usize>,
) -> Option<&'static str> {
    let key = animal.to_ascii_lowercase();
    let directory = IMAGES.get_dir(Path::new("animals").join(&key))?;
    let mut images = directory
        .files()
        .filter(|file| {
            file.path()
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
        })
        .filter_map(|file| file.path().to_str())
        .collect::<Vec<_>>();
    images.sort_by_key(|path| path.to_ascii_lowercase());
    if images.is_empty() {
        return None;
    }

    let cycle = cycles.entry(key).or_default();
    let image = images[*cycle % images.len()];
    *cycle = (*cycle + 1) % images.len();
    Some(image)
}
