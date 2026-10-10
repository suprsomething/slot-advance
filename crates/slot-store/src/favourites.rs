//! The carts starred into the Favourites category.
//!
//! One cart per line in `Config/favourites.txt`, as `<platform dir>/<rom stem>`, eg
//! `GBA/Mother 3 (Japan)`. The platform keeps a GBA and a Game Boy cart that share a stem
//! apart. The file is plain text so it can be edited on a computer; blank lines and lines
//! that name no known platform are ignored.

use std::collections::BTreeSet;
use std::path::Path;

use crate::{atomic_write, Platform};

pub const FAVOURITES_FILE: &str = "Config/favourites.txt";

pub fn favourite_key(platform: Platform, stem: &str) -> String {
    format!("{}/{stem}", platform.dir_name())
}

pub fn read_favourites(root: &Path) -> BTreeSet<String> {
    let Ok(text) = std::fs::read_to_string(root.join(FAVOURITES_FILE)) else {
        return BTreeSet::new();
    };
    text.lines()
        .map(|l| l.trim_end_matches('\r'))
        .filter(|l| {
            l.split_once('/').is_some_and(|(dir, stem)| {
                !stem.trim().is_empty() && Platform::ALL.iter().any(|p| p.dir_name() == dir)
            })
        })
        .map(str::to_string)
        .collect()
}

pub fn write_favourites(root: &Path, favourites: &BTreeSet<String>) -> std::io::Result<()> {
    let mut text = String::new();
    for key in favourites {
        text.push_str(key);
        text.push('\n');
    }
    let path = root.join(FAVOURITES_FILE);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    atomic_write(&path, text.as_bytes())
}
