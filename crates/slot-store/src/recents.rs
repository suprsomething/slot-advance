//! The carts played most recently, newest first, for the Recents category.
//!
//! Adapted from ciscosweater/slots (`slot-store/src/recents.rs`): the same newest-first,
//! de-duplicated, capped list, with two changes. It lives in `Config/recents.txt`, beside
//! the other settings that survive replacing `System/` on an update, and each line is a
//! `<platform dir>/<rom stem>` key like `Config/favourites.txt`, eg `GBA/Mother 3 (Japan)`.
//! Games that are not on the card right now keep their place, so taking a rom off for a
//! while does not erase its history.

use std::path::Path;

use crate::atomic_write;

pub const RECENTS_FILE: &str = "Config/recents.txt";
pub const RECENTS_MAX: usize = 10;

pub fn read_recents(root: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(root.join(RECENTS_FILE)) else {
        return Vec::new();
    };
    let mut recents: Vec<String> = Vec::new();
    for key in text.lines().map(|l| l.trim_end_matches('\r')) {
        let valid = key
            .split_once('/')
            .is_some_and(|(dir, stem)| !dir.is_empty() && !stem.trim().is_empty());
        if valid && !recents.iter().any(|k| k == key) {
            recents.push(key.to_string());
        }
        if recents.len() == RECENTS_MAX {
            break;
        }
    }
    recents
}

pub fn write_recents(root: &Path, recents: &[String]) -> std::io::Result<()> {
    let mut text = String::new();
    for key in recents.iter().take(RECENTS_MAX) {
        text.push_str(key);
        text.push('\n');
    }
    let path = root.join(RECENTS_FILE);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    atomic_write(&path, text.as_bytes())
}

/// Moves `key` to the front without duplicates and keeps the list capped.
pub fn touch_recent(recents: &mut Vec<String>, key: &str) {
    recents.retain(|k| k != key);
    recents.insert(0, key.to_string());
    recents.truncate(RECENTS_MAX);
}
