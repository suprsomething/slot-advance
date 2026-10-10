use std::path::{Path, PathBuf};

use crate::atomic::{atomic_write, sync_dir};
use crate::core::Core;
use crate::platform::Platform;

/// Manual states kept per cart. Each SELECT + R1 adds one; a sixth pushes out the oldest.
pub const RING_MAX: usize = 5;

/// Where states beyond the ring go when a save would push out more than one at once. That
/// only happens to a cart saved under the old ten-deep ring, so those states are moved here,
/// out of the switcher, instead of being deleted.
pub const OLDER_DIR: &str = "older";

const STATE_EXT: &str = "state";
const THUMB_EXT: &str = "png";
const RESUME: &str = "resume";
const REFUSED: &str = "resume-refused";

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StateEntry {
    pub stamp: String,
    pub state: PathBuf,
    pub thumb: PathBuf,
}

pub struct StateRing {
    dir: PathBuf,
}

impl StateRing {
    pub fn new(root: &Path, platform: Platform, core: Core, stem: &str) -> Self {
        StateRing {
            dir: root
                .join("States")
                .join(platform.dir_name())
                .join(core.as_str())
                .join(stem),
        }
    }

    pub fn push(&self, state: &[u8], thumb_png: &[u8], stamp: &str) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        atomic_write(&self.path(stamp, THUMB_EXT), thumb_png)?;
        atomic_write(&self.path(stamp, STATE_EXT), state)?;
        self.evict()
    }

    pub fn list(&self) -> std::io::Result<Vec<StateEntry>> {
        let dir = match std::fs::read_dir(&self.dir) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };

        let mut entries = Vec::new();
        for entry in dir {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some(STATE_EXT) {
                continue;
            }
            let Some(stamp) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if !is_stamp(stamp) {
                continue;
            }
            entries.push(StateEntry {
                thumb: self.path(stamp, THUMB_EXT),
                state: self.path(stamp, STATE_EXT),
                stamp: stamp.to_string(),
            });
        }
        entries.sort_by(|a, b| b.stamp.cmp(&a.stamp));
        Ok(entries)
    }

    pub fn read(&self, stamp: &str) -> std::io::Result<(Vec<u8>, Vec<u8>)> {
        let state = std::fs::read(self.stamped(stamp, STATE_EXT)?)?;
        let thumb = std::fs::read(self.path(stamp, THUMB_EXT)).unwrap_or_default();
        Ok((state, thumb))
    }

    pub fn remove(&self, stamp: &str) -> std::io::Result<()> {
        std::fs::remove_file(self.stamped(stamp, STATE_EXT)?)?;
        let _ = std::fs::remove_file(self.path(stamp, THUMB_EXT));
        Ok(())
    }

    pub fn write_resume(&self, state: &[u8]) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        atomic_write(&self.path(RESUME, STATE_EXT), state)
    }

    pub fn read_resume(&self) -> std::io::Result<Option<Vec<u8>>> {
        match std::fs::read(self.path(RESUME, STATE_EXT)) {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn retire_resume(&self, stamp: &str) -> std::io::Result<Option<PathBuf>> {
        let from = self.path(RESUME, STATE_EXT);
        if !from.exists() {
            return Ok(None);
        }
        let to = self.free_refused(stamp);
        std::fs::rename(&from, &to)?;
        sync_dir(&to);
        Ok(Some(to))
    }

    fn free_refused(&self, stamp: &str) -> PathBuf {
        let base = format!("{REFUSED}-{stamp}");
        let first = self.path(&base, STATE_EXT);
        if !first.exists() {
            return first;
        }
        (2u32..)
            .map(|n| self.path(&format!("{base}-{n}"), STATE_EXT))
            .find(|p| !p.exists())
            .unwrap_or(first)
    }

    /// Drops what no longer fits. One state over is the ring turning: the oldest is
    /// deleted. More than one over means states from the old ten-deep ring, which are moved
    /// into `older/` instead, so nothing saved before the ring shrank is ever lost.
    fn evict(&self) -> std::io::Result<()> {
        let over: Vec<StateEntry> = self.list()?.into_iter().skip(RING_MAX).collect();
        if over.len() == 1 {
            std::fs::remove_file(&over[0].state)?;
            let _ = std::fs::remove_file(&over[0].thumb);
            return Ok(());
        }
        if over.is_empty() {
            return Ok(());
        }
        let older = self.dir.join(OLDER_DIR);
        std::fs::create_dir_all(&older)?;
        for old in over {
            let (state, thumb) = free_pair(&older, &old.stamp);
            if old.thumb.exists() {
                std::fs::rename(&old.thumb, &thumb)?;
            }
            std::fs::rename(&old.state, &state)?;
        }
        sync_dir(&older.join("."));
        sync_dir(&self.dir.join("."));
        Ok(())
    }

    /// The states moved out of the ring by `evict`, newest first. Not offered in the
    /// switcher; kept so a person can recover one from a card reader.
    pub fn older(&self) -> Vec<PathBuf> {
        let Ok(dir) = std::fs::read_dir(self.dir.join(OLDER_DIR)) else {
            return Vec::new();
        };
        let mut out: Vec<PathBuf> = dir
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some(STATE_EXT))
            .collect();
        out.sort();
        out.reverse();
        out
    }

    fn path(&self, stem: &str, ext: &str) -> PathBuf {
        self.dir.join(format!("{stem}.{ext}"))
    }

    fn stamped(&self, stamp: &str, ext: &str) -> std::io::Result<PathBuf> {
        if !is_stamp(stamp) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("not a state stamp: {stamp}"),
            ));
        }
        Ok(self.path(stamp, ext))
    }
}

fn is_stamp(s: &str) -> bool {
    const SHAPE: &[u8] = b"0000-00-00_00-00-00";
    s.len() == SHAPE.len()
        && s.bytes().zip(SHAPE).all(|(c, shape)| match shape {
            b'0' => c.is_ascii_digit(),
            _ => c == *shape,
        })
}

/// A `<stamp>.state` / `<stamp>.png` pair in `dir` that does not overwrite anything there.
fn free_pair(dir: &Path, stamp: &str) -> (PathBuf, PathBuf) {
    let pair = |name: &str| {
        (
            dir.join(format!("{name}.{STATE_EXT}")),
            dir.join(format!("{name}.{THUMB_EXT}")),
        )
    };
    let first = pair(stamp);
    if !first.0.exists() && !first.1.exists() {
        return first;
    }
    (2u32..)
        .map(|n| pair(&format!("{stamp}-{n}")))
        .find(|(s, t)| !s.exists() && !t.exists())
        .unwrap_or(first)
}
