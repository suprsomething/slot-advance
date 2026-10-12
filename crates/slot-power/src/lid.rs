//! The lid, read so that a stale "shut" after a wake cannot keep the screen dark.
//!
//! The lid's driver keeps its state in `hallkey` (power supply attribute: 1 open, 0 shut).
//! On the RG SP (2026) that value is updated by the driver's own work functions. On every
//! resume the driver first records "released" (0) and only then, in a second step, "open"
//! (1). Seen on 0.4.0-beta.1: after a quick lid flick, the second step sometimes never
//! came, so `hallkey` read 0 with the lid open, and slot advance put the device back to
//! sleep. It stayed dark for several seconds, until a later suspend attempt refreshed it.
//!
//! The sensor's raw level is also visible, in the GPIO table under debugfs (the line
//! claimed as "hall switch"). That is the physical state, whatever the driver last
//! recorded. Its polarity is not documented, so it is learnt: the first time `hallkey`
//! reads 1 (which is never stale in the wrong direction) with a steady raw level, that
//! level is taken as "open".
//!
//! Reading: `hallkey` 1 is trusted as open. `hallkey` 0 is confirmed against the raw level
//! once calibrated, and the raw level wins. Without the GPIO table, or before calibration,
//! `hallkey` alone decides, exactly as before.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

/// The GPIO table line the lid's sensor is claimed under.
const HALL_LABEL: &str = "|hall switch";

/// Gap between the two raw reads that must agree before the polarity is learnt.
const STEADY_GAP: Duration = Duration::from_millis(20);

pub struct LidSensor {
    hall: PathBuf,
    gpio: Option<PathBuf>,
    /// The raw level (true = high) that means open, once learnt.
    open_level: Mutex<Option<bool>>,
}

/// What each source said, for the log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LidReading {
    pub hallkey: Option<bool>,
    pub raw_high: Option<bool>,
    pub open: Option<bool>,
}

impl LidReading {
    pub fn describe(&self) -> String {
        let h = match self.hallkey {
            Some(true) => "open",
            Some(false) => "shut",
            None => "?",
        };
        let r = match self.raw_high {
            Some(true) => "hi",
            Some(false) => "lo",
            None => "?",
        };
        format!("hallkey {h}, sensor {r}")
    }
}

impl LidSensor {
    pub fn new(hall: PathBuf, gpio: Option<PathBuf>) -> LidSensor {
        LidSensor {
            hall,
            gpio,
            open_level: Mutex::new(None),
        }
    }

    /// Finds `hallkey` among the power supplies under `sys`, and the GPIO table beside it.
    pub fn find(sys: &Path) -> Option<LidSensor> {
        let mut supplies: Vec<PathBuf> = fs::read_dir(sys.join("class/power_supply"))
            .ok()?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect();
        supplies.sort();
        let hall = supplies
            .into_iter()
            .map(|d| d.join("hallkey"))
            .find(|p| p.is_file())?;
        Some(LidSensor::new(hall, Some(sys.join("kernel/debug/gpio"))))
    }

    /// Whether the lid is open, or None if `hallkey` cannot be read.
    pub fn open(&self) -> Option<bool> {
        self.read().open
    }

    pub fn read(&self) -> LidReading {
        let hallkey = self.hallkey();
        if hallkey == Some(true) {
            // Trusted. Learn the sensor's polarity from it if not yet known.
            let raw_high = self.learn();
            return LidReading {
                hallkey,
                raw_high,
                open: Some(true),
            };
        }
        let raw_high = self.raw_high();
        let level = *self.open_level.lock().unwrap_or_else(|e| e.into_inner());
        let open = match (hallkey, raw_high, level) {
            (_, Some(raw), Some(open_level)) => Some(raw == open_level),
            (h, _, _) => h,
        };
        LidReading {
            hallkey,
            raw_high,
            open,
        }
    }

    fn learn(&self) -> Option<bool> {
        let mut level = self.open_level.lock().unwrap_or_else(|e| e.into_inner());
        if level.is_some() {
            return None;
        }
        let first = self.raw_high()?;
        std::thread::sleep(STEADY_GAP);
        let second = self.raw_high()?;
        if first == second && self.hallkey() == Some(true) {
            *level = Some(first);
            eprintln!(
                "slot: lid: sensor reads {} with the lid open",
                if first { "hi" } else { "lo" }
            );
        }
        Some(second)
    }

    fn hallkey(&self) -> Option<bool> {
        match fs::read_to_string(&self.hall).ok()?.trim() {
            "1" => Some(true),
            "0" => Some(false),
            _ => None,
        }
    }

    fn raw_high(&self) -> Option<bool> {
        let table = fs::read_to_string(self.gpio.as_ref()?).ok()?;
        raw_level(&table)
    }
}

/// The level of the line claimed as the hall switch in a debugfs GPIO table, e.g.
/// ` gpio-135 (                    |hall switch         ) in  hi    `.
pub fn raw_level(table: &str) -> Option<bool> {
    let line = table.lines().find(|l| l.contains(HALL_LABEL))?;
    let after = &line[line.rfind(')')? + 1..];
    let mut words = after.split_whitespace();
    if words.next()? != "in" {
        return None;
    }
    match words.next()? {
        "hi" => Some(true),
        "lo" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Rig {
        _dir: tempfile::TempDir,
        hall: PathBuf,
        gpio: PathBuf,
    }

    impl Rig {
        fn new() -> Rig {
            let dir = tempfile::tempdir().unwrap();
            let hall = dir.path().join("hallkey");
            let gpio = dir.path().join("gpio");
            Rig {
                _dir: dir,
                hall,
                gpio,
            }
        }

        fn set(&self, hallkey: &str, raw: &str) {
            fs::write(&self.hall, format!("{hallkey}\n")).unwrap();
            fs::write(
                &self.gpio,
                format!(
                    "gpiochip1: GPIOs 0-287, parent: platform/pio, pio:\n \
                     gpio-135 (                    |hall switch         ) in  {raw}    \n \
                     gpio-147 (                    |?                   ) out hi    \n"
                ),
            )
            .unwrap();
        }

        fn sensor(&self) -> LidSensor {
            LidSensor::new(self.hall.clone(), Some(self.gpio.clone()))
        }
    }

    #[test]
    fn the_hall_switch_line_is_parsed() {
        let t = " gpio-135 (                    |hall switch         ) in  hi    \n";
        assert_eq!(raw_level(t), Some(true));
        assert_eq!(raw_level(&t.replace(" hi", " lo")), Some(false));
        assert_eq!(raw_level(" gpio-147 (  |?  ) out hi\n"), None);
        assert_eq!(raw_level(""), None);
    }

    #[test]
    fn a_stale_shut_after_a_wake_is_overruled_by_the_sensor() {
        let r = Rig::new();
        let lid = r.sensor();
        r.set("1", "hi");
        assert_eq!(lid.open(), Some(true), "calibrates: hi is open");
        r.set("0", "lo");
        assert_eq!(lid.open(), Some(false), "a real close");
        r.set("0", "hi");
        assert_eq!(
            lid.open(),
            Some(true),
            "hallkey stuck at 0 with the lid open: the sensor wins"
        );
        r.set("0", "lo");
        assert_eq!(lid.open(), Some(false), "closing again is still seen");
    }

    #[test]
    fn the_polarity_is_learnt_not_assumed() {
        let r = Rig::new();
        let lid = r.sensor();
        r.set("1", "lo");
        assert_eq!(lid.open(), Some(true));
        r.set("0", "lo");
        assert_eq!(lid.open(), Some(true), "lo was learnt as open");
        r.set("0", "hi");
        assert_eq!(lid.open(), Some(false));
    }

    #[test]
    fn before_calibration_hallkey_alone_decides() {
        let r = Rig::new();
        let lid = r.sensor();
        r.set("0", "hi");
        assert_eq!(lid.open(), Some(false), "booted with the lid shut");
        r.set("1", "hi");
        assert_eq!(lid.open(), Some(true));
        r.set("0", "lo");
        assert_eq!(lid.open(), Some(false));
    }

    #[test]
    fn without_the_gpio_table_it_is_hallkey_as_before() {
        let r = Rig::new();
        r.set("1", "hi");
        let lid = LidSensor::new(r.hall.clone(), None);
        assert_eq!(lid.open(), Some(true));
        fs::write(&r.hall, "0\n").unwrap();
        assert_eq!(lid.open(), Some(false));
        fs::write(&r.hall, "?\n").unwrap();
        assert_eq!(lid.open(), None);
    }

    #[test]
    fn an_unreadable_sensor_falls_back_to_hallkey() {
        let r = Rig::new();
        let lid = r.sensor();
        r.set("1", "hi");
        assert_eq!(lid.open(), Some(true));
        fs::write(&r.hall, "0\n").unwrap();
        fs::write(&r.gpio, "no hall line here\n").unwrap();
        assert_eq!(lid.open(), Some(false));
    }
}
