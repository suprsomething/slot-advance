#[derive(Copy, Clone, Debug)]
pub struct Build {
    pub version: &'static str,
    pub hash: &'static str,
    pub dirty: bool,
    pub date: &'static str,
}

impl Build {
    pub fn current() -> Build {
        Build {
            version: env!("CARGO_PKG_VERSION"),
            hash: env!("SLOT_GIT_HASH"),
            dirty: env!("SLOT_GIT_DIRTY") == "1",
            date: env!("SLOT_BUILD_DATE"),
        }
    }

    /// One line naming this build exactly, for the top of slot.log: the version, the
    /// short Git commit it was built from (marked if built with uncommitted changes) and
    /// the build date.
    pub fn describe(&self) -> String {
        let dirty = if self.dirty { "+changes" } else { "" };
        format!(
            "slot advance {} ({}{dirty}, built {})",
            self.version, self.hash, self.date
        )
    }

    pub fn serial(&self) -> String {
        self.hash.to_uppercase()
    }

    pub fn dirty_digit(&self) -> char {
        match self.dirty {
            true => '1',
            false => '0',
        }
    }
}
