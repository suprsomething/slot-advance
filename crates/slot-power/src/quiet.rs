//! Switches off hardware slot advance never uses, once per session, in the background.
//!
//! - The six USB host controllers (`ohci1-3`, `ehci1-3`): nothing is wired to them on the
//!   RG SP. The USB-C port has its own controller and is not touched. Measured in
//!   Experiment 5 as the largest single saving while suspended.
//! - The Wi-Fi radio: BaseOS loads its driver and powers the radio at every boot even
//!   though nothing joins a network. Unloading the driver cuts its supply rail (verified in
//!   Experiment 5). Skipped entirely when `wifi=on` is set, so SlotLink keeps working.
//!
//! Both are optional. A failure is logged and changes nothing else: hardware sleep never
//! depends on either. Nothing here outlives the session; BaseOS restores both at boot.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// OHCI first, then EHCI, the order that was tested on the device.
pub const IDLE_CONTROLLERS: [(&str, &str); 6] = [
    ("5200000.ohci1-controller", "ohci_enable"),
    ("5310000.ohci2-controller", "ohci_enable"),
    ("5311000.ohci3-controller", "ohci_enable"),
    ("5200000.ehci1-controller", "ehci_enable"),
    ("5310000.ehci2-controller", "ehci_enable"),
    ("5311000.ehci3-controller", "ehci_enable"),
];

/// BaseOS brings Wi-Fi up in the background and may retry a slow radio until about 17 s
/// after boot. Unloading the driver before then could race its retry.
pub const WIFI_SETTLED_UPTIME: f64 = 20.0;

const WIFI_WAIT_LIMIT: Duration = Duration::from_secs(90);
const RMMOD_LIMIT: Duration = Duration::from_secs(15);
const WIFI_DRIVER: &str = "8821cs";

pub struct Quiet {
    pub sys: PathBuf,
    pub proc: PathBuf,
    pub run: PathBuf,
    pub rmmod: PathBuf,
    /// Leave Wi-Fi alone (`wifi=on`).
    pub keep_wifi: bool,
    /// How often to check uptime while waiting for BaseOS's Wi-Fi start to settle.
    pub poll: Duration,
}

impl Quiet {
    pub fn device(keep_wifi: bool) -> Quiet {
        Quiet {
            sys: PathBuf::from("/sys"),
            proc: PathBuf::from("/proc"),
            run: PathBuf::from("/run"),
            rmmod: PathBuf::from("/sbin/rmmod"),
            keep_wifi,
            poll: Duration::from_secs(1),
        }
    }

    /// Switches off what it can, logging each step. Blocks until done (up to about two
    /// minutes while waiting for Wi-Fi to settle), so call it from a background thread.
    pub fn run(&self) -> QuietReport {
        let usb = self.usb_off();
        let wifi = if self.keep_wifi {
            eprintln!("slot: quiet: wifi=on, Wi-Fi left as BaseOS started it");
            WifiResult::Kept
        } else {
            self.wifi_off()
        };
        QuietReport { usb, wifi }
    }

    /// Returns how many controllers are now off.
    pub fn usb_off(&self) -> usize {
        let soc = self.sys.join("devices/platform/soc");
        let mut off = 0;
        for (dev, file) in IDLE_CONTROLLERS {
            let switch = soc.join(dev).join(file);
            let name = dev
                .split('.')
                .nth(1)
                .unwrap_or(dev)
                .trim_end_matches("-controller");
            match controller_on(&switch) {
                None => {
                    eprintln!("slot: quiet: usb {name}: no switch, left alone");
                    continue;
                }
                Some(false) => {
                    off += 1;
                    continue;
                }
                Some(true) => {}
            }
            if let Err(e) = fs::write(&switch, "0") {
                eprintln!("slot: quiet: usb {name}: {e}");
                continue;
            }
            std::thread::sleep(Duration::from_millis(100));
            match controller_on(&switch) {
                Some(false) => off += 1,
                _ => eprintln!("slot: quiet: usb {name}: did not switch off"),
            }
        }
        eprintln!(
            "slot: quiet: usb host controllers off: {off} of {}",
            IDLE_CONTROLLERS.len()
        );
        off
    }

    pub fn wifi_off(&self) -> WifiResult {
        let began = Instant::now();
        while self.uptime().is_some_and(|u| u < WIFI_SETTLED_UPTIME) {
            if began.elapsed() > WIFI_WAIT_LIMIT {
                break;
            }
            std::thread::sleep(self.poll);
        }
        if self.run.join("slotlink.session").exists() {
            eprintln!("slot: quiet: a SlotLink session is open, Wi-Fi left alone");
            return WifiResult::Kept;
        }
        if !self.driver_loaded() {
            let off = self.radio_off();
            eprintln!("slot: quiet: wifi driver not loaded (radio off: {off})");
            return if off {
                WifiResult::Off
            } else {
                WifiResult::Failed
            };
        }
        match run_limited(&self.rmmod, WIFI_DRIVER, RMMOD_LIMIT) {
            Ok(true) => {}
            Ok(false) => {
                eprintln!("slot: quiet: rmmod {WIFI_DRIVER} failed, Wi-Fi left on");
                return WifiResult::Failed;
            }
            Err(e) => {
                eprintln!("slot: quiet: rmmod {WIFI_DRIVER}: {e}");
                return WifiResult::Failed;
            }
        }
        let until = Instant::now() + Duration::from_secs(3);
        while !self.radio_off() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(100));
        }
        if self.radio_off() {
            eprintln!("slot: quiet: wifi off (driver unloaded, radio supply off)");
            WifiResult::Off
        } else {
            eprintln!("slot: quiet: wifi driver unloaded but the radio still reports power");
            WifiResult::Failed
        }
    }

    fn uptime(&self) -> Option<f64> {
        fs::read_to_string(self.proc.join("uptime"))
            .ok()?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    }

    fn driver_loaded(&self) -> bool {
        fs::read_to_string(self.proc.join("modules")).is_ok_and(|m| {
            m.lines()
                .any(|l| l.split_whitespace().next() == Some(WIFI_DRIVER))
        })
    }

    /// The radio counts as off when the driver is gone, `wlan0` is gone and the sunxi
    /// rf-ctrl power state does not read 1.
    pub fn radio_off(&self) -> bool {
        let power = fs::read_to_string(self.sys.join("class/misc/sunxi-wlan/rf-ctrl/power_state"))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        !self.driver_loaded() && !self.sys.join("class/net/wlan0").exists() && power != "1"
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WifiResult {
    Off,
    Kept,
    Failed,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct QuietReport {
    pub usb: usize,
    pub wifi: WifiResult,
}

/// Starts the quiet job on its own thread.
pub fn quiet_hardware(quiet: Quiet) {
    let spawned = std::thread::Builder::new()
        .name("slot-quiet".into())
        .spawn(move || {
            quiet.run();
        });
    if let Err(e) = spawned {
        eprintln!("slot: quiet: no thread: {e}");
    }
}

/// `Some(true)` when the controller is on, `Some(false)` when off, `None` with no switch.
/// The kernel shows `ehci:1, probe:1`; the last number is whether it is running.
fn controller_on(switch: &Path) -> Option<bool> {
    let v = fs::read_to_string(switch).ok()?;
    let v = v.trim();
    let last = v.rsplit(':').next().unwrap_or(v).trim();
    Some(last != "0")
}

fn run_limited(program: &Path, arg: &str, limit: Duration) -> std::io::Result<bool> {
    let mut child = Command::new(program).arg(arg).spawn()?;
    let until = Instant::now() + limit;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status.success());
        }
        if Instant::now() >= until {
            let _ = child.kill();
            let _ = child.wait();
            return Err(std::io::Error::other("timed out"));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Mock {
        dir: tempfile::TempDir,
    }

    impl Mock {
        fn new(wifi: bool) -> Mock {
            let dir = tempfile::tempdir().unwrap();
            let sys = dir.path().join("sys");
            let soc = sys.join("devices/platform/soc");
            for (dev, file) in IDLE_CONTROLLERS {
                fs::create_dir_all(soc.join(dev)).unwrap();
                let n = &dev[dev.len() - 12..dev.len() - 11];
                let kind = &file[..4];
                fs::write(soc.join(dev).join(file), format!("{kind}:{n}, probe:1\n")).unwrap();
            }
            let proc = dir.path().join("proc");
            fs::create_dir_all(&proc).unwrap();
            fs::write(proc.join("uptime"), "25.00 1.00\n").unwrap();
            let rf = sys.join("class/misc/sunxi-wlan/rf-ctrl");
            fs::create_dir_all(&rf).unwrap();
            if wifi {
                fs::write(proc.join("modules"), "8821cs 2420000 0 - Live 0x0\n").unwrap();
                fs::create_dir_all(sys.join("class/net/wlan0")).unwrap();
                fs::write(rf.join("power_state"), "1\n").unwrap();
            } else {
                fs::write(proc.join("modules"), "mali_kbase 1 3 - Live 0x0\n").unwrap();
                fs::write(rf.join("power_state"), "0\n").unwrap();
            }
            fs::create_dir_all(dir.path().join("run")).unwrap();
            Mock { dir }
        }

        fn quiet(&self, rmmod: &str, keep_wifi: bool) -> Quiet {
            let p = self.dir.path();
            let script = p.join("rmmod");
            fs::write(&script, rmmod).unwrap();
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
            Quiet {
                sys: p.join("sys"),
                proc: p.join("proc"),
                run: p.join("run"),
                rmmod: script,
                keep_wifi,
                poll: Duration::from_millis(5),
            }
        }
    }

    /// A stand-in for the kernel: unloading the driver takes the radio down with it.
    fn good_rmmod(m: &Mock) -> String {
        let p = m.dir.path().display();
        format!(
            "#!/bin/sh\n[ \"$1\" = 8821cs ] || exit 1\nrm -rf {p}/sys/class/net/wlan0\n\
             echo 0 > {p}/sys/class/misc/sunxi-wlan/rf-ctrl/power_state\n\
             echo 'mali_kbase 1 3 - Live 0x0' > {p}/proc/modules\n"
        )
    }

    #[test]
    fn all_six_controllers_switch_off() {
        let m = Mock::new(false);
        let q = m.quiet("#!/bin/sh\nexit 1\n", false);
        assert_eq!(q.usb_off(), 6);
        for (dev, file) in IDLE_CONTROLLERS {
            let v = fs::read_to_string(q.sys.join("devices/platform/soc").join(dev).join(file))
                .unwrap();
            assert_eq!(v.trim(), "0", "{dev}");
        }
    }

    #[test]
    fn a_missing_controller_is_skipped_and_the_rest_still_go_off() {
        let m = Mock::new(false);
        let q = m.quiet("#!/bin/sh\nexit 1\n", false);
        fs::remove_dir_all(q.sys.join("devices/platform/soc/5310000.ehci2-controller")).unwrap();
        assert_eq!(q.usb_off(), 5);
    }

    #[test]
    fn wifi_goes_off_and_is_checked() {
        let m = Mock::new(true);
        let q = m.quiet(&good_rmmod(&m), false);
        assert_eq!(q.wifi_off(), WifiResult::Off);
        assert!(q.radio_off());
    }

    #[test]
    fn wifi_on_leaves_the_radio_alone() {
        let m = Mock::new(true);
        let q = m.quiet("#!/bin/sh\necho touched > /dev/null; exit 9\n", true);
        assert_eq!(q.run().wifi, WifiResult::Kept);
        assert!(!q.radio_off(), "the radio was touched with wifi=on");
    }

    #[test]
    fn an_open_slotlink_session_is_never_interrupted() {
        let m = Mock::new(true);
        let q = m.quiet(&good_rmmod(&m), false);
        fs::write(q.run.join("slotlink.session"), "123\n").unwrap();
        assert_eq!(q.wifi_off(), WifiResult::Kept);
        assert!(!q.radio_off());
    }

    #[test]
    fn it_waits_for_baseos_to_finish_starting_wifi() {
        let m = Mock::new(true);
        let q = m.quiet(&good_rmmod(&m), false);
        fs::write(q.proc.join("uptime"), "3.00 1.00\n").unwrap();
        let uptime = q.proc.join("uptime");
        let later = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            fs::write(uptime, "21.00 1.00\n").unwrap();
        });
        let began = Instant::now();
        assert_eq!(q.wifi_off(), WifiResult::Off);
        assert!(
            began.elapsed() >= Duration::from_millis(90),
            "it did not wait"
        );
        later.join().unwrap();
    }

    #[test]
    fn a_failing_rmmod_is_reported_not_fatal() {
        let m = Mock::new(true);
        let q = m.quiet("#!/bin/sh\nexit 1\n", false);
        assert_eq!(q.wifi_off(), WifiResult::Failed);
    }

    #[test]
    fn already_off_is_off() {
        let m = Mock::new(false);
        let q = m.quiet("#!/bin/sh\nexit 1\n", false);
        assert_eq!(q.wifi_off(), WifiResult::Off);
    }
}
