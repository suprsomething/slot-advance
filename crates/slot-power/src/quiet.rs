//! Switches off hardware slot advance never uses, once per session, in the background.
//!
//! - The six USB host controllers (`ohci1-3`, `ehci1-3`): nothing is wired to them on the
//!   RG SP. The USB-C port has its own controller and is not touched. Measured in
//!   Experiment 5 as the largest single saving while suspended.
//! - The Wi-Fi radio: BaseOS loads its driver and powers the radio at every boot even
//!   though nothing joins a network. Unloading the driver cuts its supply rail (verified in
//!   Experiment 5). Skipped entirely when `wifi=on` is set, so SlotLink keeps working.
//! - The Wi-Fi radio's stale SDIO card (0.4.1): unloading the driver leaves the powered-off
//!   card registered with the kernel, and every resume then waits about 1.1 s for it to time
//!   out (`mmc2: error -110`). Straight after its own unload, and only then, the quiet job
//!   asks the sunxi Wi-Fi driver to rescan the empty slot (`rf-ctrl/scan_device`, the step
//!   BaseOS's start-up script uses), which removes the card. Measured in experiment R1a:
//!   gone in 40 ms, device resume 1.1 s down to about 655 ms, across 14 consecutive
//!   suspends on three boots, with both TF cards untouched and the radio unpowered.
//!   Every safety check must pass first; otherwise the card is left exactly as in 0.4.0.
//!
//! All are optional. A failure is logged and changes nothing else: hardware sleep never
//! depends on any of them. Nothing here outlives the session; BaseOS restores all of it at
//! boot.

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

/// Writing 1 makes the sunxi Wi-Fi driver rescan its SDIO slot. With the radio unpowered
/// the rescan finds nothing and the kernel removes the stale card.
const SCAN_DEVICE: &str = "class/misc/sunxi-wlan/rf-ctrl/scan_device";
/// The Wi-Fi radio's supply rail, as Experiment 5 and R1a read it.
const WLAN_RAIL: &str = "axp2202-cldo4";
/// R1a saw the card go within 40 ms every time.
const DISCARD_LIMIT: Duration = Duration::from_secs(3);

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
        self.unload_wifi().0
    }

    /// Unloads the driver as 0.4.0 does, then, only if that unload succeeded, discards the
    /// stale SDIO card. Returns what happened to each.
    pub fn unload_wifi(&self) -> (WifiResult, Option<StaleCard>) {
        let began = Instant::now();
        while self.uptime().is_some_and(|u| u < WIFI_SETTLED_UPTIME) {
            if began.elapsed() > WIFI_WAIT_LIMIT {
                break;
            }
            std::thread::sleep(self.poll);
        }
        if self.run.join("slotlink.session").exists() {
            eprintln!("slot: quiet: a SlotLink session is open, Wi-Fi left alone");
            return (WifiResult::Kept, None);
        }
        if !self.driver_loaded() {
            let off = self.radio_off();
            eprintln!("slot: quiet: wifi driver not loaded (radio off: {off})");
            let result = if off {
                WifiResult::Off
            } else {
                WifiResult::Failed
            };
            return (result, None);
        }
        // Which slot holds the Wi-Fi card, worked out while the driver still has it.
        let host = self.wifi_host();
        match run_limited(&self.rmmod, WIFI_DRIVER, RMMOD_LIMIT) {
            Ok(true) => {}
            Ok(false) => {
                eprintln!("slot: quiet: rmmod {WIFI_DRIVER} failed, Wi-Fi left on");
                return (WifiResult::Failed, None);
            }
            Err(e) => {
                eprintln!("slot: quiet: rmmod {WIFI_DRIVER}: {e}");
                return (WifiResult::Failed, None);
            }
        }
        let until = Instant::now() + Duration::from_secs(3);
        while !self.radio_off() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(100));
        }
        if self.radio_off() {
            eprintln!("slot: quiet: wifi off (driver unloaded, radio supply off)");
            let card = match host {
                Ok(host) => self.discard_stale_card(&host),
                Err(why) => StaleCard::Left(why),
            };
            card.log();
            (WifiResult::Off, Some(card))
        } else {
            eprintln!("slot: quiet: wifi driver unloaded but the radio still reports power");
            (WifiResult::Failed, None)
        }
    }

    /// The mmc host (`mmc2` on the RG SP) holding the Wi-Fi card, found two independent
    /// ways that must agree, and never the host of a block device (TF1 or TF2). Call it
    /// while the driver is loaded.
    ///
    /// 1. The device tree's `wlan_busnum` is the Wi-Fi's controller number (`sdc1`), which
    ///    is matched by name to exactly one mmc host. It is not the host number: on the RG
    ///    SP `mmc1` is `sdc2`, the games card.
    /// 2. Every SDIO function the kernel lists must be on that same host.
    pub fn wifi_host(&self) -> Result<String, String> {
        let base = self.sys.join("firmware/devicetree/base");
        let mut nums = Vec::new();
        let mut candidates = vec![base.join("wlan/wlan_busnum")];
        if let Ok(dir) = fs::read_dir(&base) {
            for e in dir.flatten() {
                candidates.push(e.path().join("wlan/wlan_busnum"));
            }
        }
        for c in candidates {
            if let Ok(bytes) = fs::read(&c) {
                if bytes.len() == 4 {
                    nums.push(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
                }
            }
        }
        nums.sort_unstable();
        nums.dedup();
        let bus = match nums.as_slice() {
            [n] => *n,
            [] => return Err("no wlan_busnum in the device tree".into()),
            _ => return Err(format!("conflicting wlan_busnum values {nums:?}")),
        };
        let controller = format!("sdc{bus}");
        let hosts: Vec<String> = names(&self.sys.join("class/mmc_host"))
            .into_iter()
            .filter(|h| {
                link_name(&self.sys.join("class/mmc_host").join(h).join("device")).as_deref()
                    == Some(controller.as_str())
            })
            .collect();
        let [host] = hosts.as_slice() else {
            return Err(format!(
                "controller {controller} matched {} mmc hosts, not one",
                hosts.len()
            ));
        };
        let functions = names(&self.sys.join("bus/sdio/devices"));
        if functions.is_empty() {
            return Err("no SDIO function to confirm the Wi-Fi slot".into());
        }
        if let Some(f) = functions.iter().find(|f| host_of(f) != host) {
            return Err(format!("SDIO function {f} is not on {host}"));
        }
        for b in names(&self.sys.join("block")) {
            if !b.starts_with("mmcblk") || b.contains("boot") || b.contains("rpmb") {
                continue;
            }
            let card = link_name(&self.sys.join("block").join(&b).join("device"));
            match card {
                Some(c) if host_of(&c) != host => {}
                _ => return Err(format!("{b} may share {host} with the Wi-Fi")),
            }
        }
        Ok(host.clone())
    }

    /// Removes the stale SDIO card from `host` once the radio is unpowered. Every check
    /// must pass first; any doubt leaves the card exactly as 0.4.0 does.
    pub fn discard_stale_card(&self, host: &str) -> StaleCard {
        let devices = self.sys.join("bus/mmc/devices");
        let prefix = format!("{host}:");
        let cards: Vec<String> = names(&devices)
            .into_iter()
            .filter(|c| c.starts_with(&prefix))
            .collect();
        let card = match cards.as_slice() {
            [] => return StaleCard::None,
            [c] => c.clone(),
            _ => return StaleCard::Left(format!("{} cards on {host}", cards.len())),
        };
        let kind = read_trim(&devices.join(&card).join("type")).unwrap_or_default();
        if kind != "SDIO" {
            return StaleCard::Left(format!("{card} is not an SDIO card ({kind})"));
        }
        if let Err(why) = self.radio_unpowered() {
            return StaleCard::Left(why);
        }
        let scan = self.sys.join(SCAN_DEVICE);
        if !scan.exists() {
            return StaleCard::Left("rf-ctrl/scan_device is missing".into());
        }
        let others = |list: Vec<String>| -> Vec<String> {
            list.into_iter()
                .filter(|c| !c.starts_with(&prefix))
                .collect()
        };
        let before = others(names(&devices));
        let began = Instant::now();
        // "1\n", exactly as BaseOS's rcS and experiment R1a wrote it.
        if let Err(e) = fs::write(&scan, "1\n") {
            return StaleCard::Failed(format!("scan_device: {e}"));
        }
        while devices.join(&card).exists() && began.elapsed() < DISCARD_LIMIT {
            std::thread::sleep(Duration::from_millis(10));
        }
        let ms = began.elapsed().as_millis();
        if others(names(&devices)) != before {
            return StaleCard::Failed(format!("another mmc device changed (before: {before:?})"));
        }
        if let Err(why) = self.radio_unpowered() {
            return StaleCard::Failed(format!("after the rescan, {why}"));
        }
        if devices.join(&card).exists() {
            return StaleCard::Failed(format!(
                "{card} is still there {} s after the rescan",
                DISCARD_LIMIT.as_secs()
            ));
        }
        StaleCard::Discarded { card, ms }
    }

    /// The stricter test the discard needs: `power_state` reads 0 and the WLAN rail reads
    /// disabled. An unreadable rail counts as not off.
    fn radio_unpowered(&self) -> Result<(), String> {
        let power = read_trim(&self.sys.join("class/misc/sunxi-wlan/rf-ctrl/power_state"));
        if power.as_deref() != Some("0") {
            return Err(format!("rf-ctrl power_state reads {power:?}, not 0"));
        }
        let rail = names(&self.sys.join("class/regulator"))
            .into_iter()
            .map(|r| self.sys.join("class/regulator").join(r))
            .find(|r| read_trim(&r.join("name")).as_deref() == Some(WLAN_RAIL));
        let state = rail.and_then(|r| read_trim(&r.join("state")));
        if state.as_deref() != Some("disabled") {
            return Err(format!(
                "the WLAN rail ({WLAN_RAIL}) reads {state:?}, not disabled"
            ));
        }
        Ok(())
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

/// What became of the Wi-Fi radio's stale SDIO card after the driver was unloaded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StaleCard {
    /// Removed: resumes no longer wait for it.
    Discarded { card: String, ms: u128 },
    /// No card was left on the Wi-Fi slot, so there was nothing to do.
    None,
    /// A safety check did not pass, so nothing was written: resume is as in 0.4.0.
    Left(String),
    /// The rescan was asked for but something did not go as expected.
    Failed(String),
}

impl StaleCard {
    fn log(&self) {
        match self {
            StaleCard::Discarded { card, ms } => eprintln!(
                "slot: quiet: stale wifi card {card} discarded in {ms} ms (resume no longer waits for it)"
            ),
            StaleCard::None => eprintln!("slot: quiet: no stale wifi card to discard"),
            StaleCard::Left(why) => {
                eprintln!("slot: quiet: stale wifi card left in place, resume as 0.4.0: {why}")
            }
            StaleCard::Failed(why) => eprintln!("slot: quiet: stale wifi card discard FAILED: {why}"),
        }
    }
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

/// Entry names in a directory, sorted; empty if it cannot be read.
fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .map(|d| {
            d.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// The last component of a symlink's target (sysfs `device` links).
fn link_name(link: &Path) -> Option<String> {
    let target = fs::read_link(link).ok()?;
    Some(target.file_name()?.to_string_lossy().into_owned())
}

/// `mmc2` from `mmc2:0001` or `mmc2:0001:1`.
fn host_of(name: &str) -> &str {
    name.split(':').next().unwrap_or(name)
}

fn read_trim(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
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
            fs::write(rf.join("scan_device"), "").unwrap();
            let rail = sys.join("class/regulator/regulator.12");
            fs::create_dir_all(&rail).unwrap();
            fs::write(rail.join("name"), "axp2202-cldo4\n").unwrap();
            let other = sys.join("class/regulator/regulator.3");
            fs::create_dir_all(&other).unwrap();
            fs::write(other.join("name"), "axp2202-dcdc1\n").unwrap();
            fs::write(other.join("state"), "enabled\n").unwrap();
            if wifi {
                fs::write(proc.join("modules"), "8821cs 2420000 0 - Live 0x0\n").unwrap();
                fs::create_dir_all(sys.join("class/net/wlan0")).unwrap();
                fs::write(rf.join("power_state"), "1\n").unwrap();
                fs::write(rail.join("state"), "enabled\n").unwrap();
            } else {
                fs::write(proc.join("modules"), "mali_kbase 1 3 - Live 0x0\n").unwrap();
                fs::write(rf.join("power_state"), "0\n").unwrap();
                fs::write(rail.join("state"), "disabled\n").unwrap();
            }
            // The RG SP's card slots, as R1a found them: the controller number is not the
            // host number. mmc0 = sdc0 (TF1), mmc1 = sdc2 (TF2), mmc2 = sdc1 (Wi-Fi).
            let dt = sys.join("firmware/devicetree/base/soc@03000000/wlan");
            fs::create_dir_all(&dt).unwrap();
            fs::write(dt.join("wlan_busnum"), 1u32.to_be_bytes()).unwrap();
            let soc = sys.join("devices/platform/soc@03000000");
            for (host, ctrl, card, kind) in [
                ("mmc0", "sdc0", "mmc0:0001", "SD"),
                ("mmc1", "sdc2", "mmc1:aaaa", "SD"),
                ("mmc2", "sdc1", "mmc2:0001", "SDIO"),
            ] {
                let h = soc.join(ctrl).join("mmc_host").join(host);
                fs::create_dir_all(h.join(card)).unwrap();
                fs::write(h.join(card).join("type"), format!("{kind}\n")).unwrap();
                std::os::unix::fs::symlink(soc.join(ctrl), h.join("device")).unwrap();
                fs::create_dir_all(sys.join("class/mmc_host")).unwrap();
                std::os::unix::fs::symlink(&h, sys.join("class/mmc_host").join(host)).unwrap();
                fs::create_dir_all(sys.join("bus/mmc/devices")).unwrap();
                std::os::unix::fs::symlink(h.join(card), sys.join("bus/mmc/devices").join(card))
                    .unwrap();
            }
            let wifi_card = soc.join("sdc1/mmc_host/mmc2/mmc2:0001");
            fs::create_dir_all(wifi_card.join("mmc2:0001:1")).unwrap();
            fs::create_dir_all(sys.join("bus/sdio/devices")).unwrap();
            std::os::unix::fs::symlink(
                wifi_card.join("mmc2:0001:1"),
                sys.join("bus/sdio/devices/mmc2:0001:1"),
            )
            .unwrap();
            for (blk, host, card) in [
                ("mmcblk0", "sdc0/mmc_host/mmc0", "mmc0:0001"),
                ("mmcblk1", "sdc2/mmc_host/mmc1", "mmc1:aaaa"),
            ] {
                fs::create_dir_all(sys.join("block").join(blk)).unwrap();
                std::os::unix::fs::symlink(
                    soc.join(host).join(card),
                    sys.join("block").join(blk).join("device"),
                )
                .unwrap();
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
             echo disabled > {p}/sys/class/regulator/regulator.12/state\n\
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
        let k = kernel(&m, false);
        assert_eq!(q.wifi_off(), WifiResult::Off);
        k.join().unwrap();
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

    /// A stand-in for the kernel's side of the rescan: once 1 is written to scan_device,
    /// the card on the empty slot is removed. `powers` makes it briefly power the radio,
    /// which the real hardware must never do.
    fn kernel(m: &Mock, powers: bool) -> std::thread::JoinHandle<()> {
        let sys = m.dir.path().join("sys");
        std::thread::spawn(move || {
            let scan = sys.join(SCAN_DEVICE);
            let began = Instant::now();
            while began.elapsed() < Duration::from_secs(5) {
                if fs::read_to_string(&scan).unwrap_or_default().trim() == "1" {
                    if powers {
                        fs::write(sys.join("class/misc/sunxi-wlan/rf-ctrl/power_state"), "1\n")
                            .unwrap();
                    }
                    std::thread::sleep(Duration::from_millis(30));
                    let _ = fs::remove_file(sys.join("bus/mmc/devices/mmc2:0001"));
                    let _ = fs::remove_file(sys.join("bus/sdio/devices/mmc2:0001:1"));
                    return;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        })
    }

    fn card_present(q: &Quiet, card: &str) -> bool {
        q.sys.join("bus/mmc/devices").join(card).exists()
    }

    fn scan_written(q: &Quiet) -> bool {
        fs::read_to_string(q.sys.join(SCAN_DEVICE))
            .unwrap_or_default()
            .trim()
            == "1"
    }

    fn tf_cards_untouched(q: &Quiet) {
        assert!(card_present(q, "mmc0:0001"), "TF1 card gone");
        assert!(card_present(q, "mmc1:aaaa"), "TF2 card gone");
    }

    #[test]
    fn the_wifi_host_comes_from_its_controller_not_its_number() {
        let m = Mock::new(true);
        let q = m.quiet(&good_rmmod(&m), false);
        // wlan_busnum is 1, but mmc1 is the games card: the Wi-Fi is sdc1, which is mmc2.
        assert_eq!(q.wifi_host(), Ok("mmc2".to_string()));
    }

    #[test]
    fn the_stale_card_is_discarded_after_the_unload() {
        let m = Mock::new(true);
        let q = m.quiet(&good_rmmod(&m), false);
        let k = kernel(&m, false);
        let (wifi, card) = q.unload_wifi();
        k.join().unwrap();
        assert_eq!(wifi, WifiResult::Off);
        assert!(
            matches!(card, Some(StaleCard::Discarded { ref card, .. }) if card == "mmc2:0001"),
            "{card:?}"
        );
        assert!(!card_present(&q, "mmc2:0001"));
        tf_cards_untouched(&q);
        assert!(q.radio_off());
    }

    #[test]
    fn wifi_on_never_touches_the_card() {
        let m = Mock::new(true);
        let q = m.quiet(&good_rmmod(&m), true);
        assert_eq!(q.run().wifi, WifiResult::Kept);
        assert!(!scan_written(&q));
        assert!(card_present(&q, "mmc2:0001"));
    }

    #[test]
    fn an_open_slotlink_session_never_touches_the_card() {
        let m = Mock::new(true);
        let q = m.quiet(&good_rmmod(&m), false);
        fs::write(q.run.join("slotlink.session"), "123\n").unwrap();
        assert_eq!(q.unload_wifi(), (WifiResult::Kept, None));
        assert!(!scan_written(&q));
        assert!(card_present(&q, "mmc2:0001"));
    }

    #[test]
    fn a_failed_unload_never_touches_the_card() {
        let m = Mock::new(true);
        let q = m.quiet("#!/bin/sh\nexit 1\n", false);
        assert_eq!(q.unload_wifi(), (WifiResult::Failed, None));
        assert!(!scan_written(&q));
        assert!(card_present(&q, "mmc2:0001"));
    }

    #[test]
    fn a_driver_that_was_never_loaded_leaves_the_card_alone() {
        let m = Mock::new(false);
        let q = m.quiet("#!/bin/sh\nexit 1\n", false);
        assert_eq!(q.unload_wifi(), (WifiResult::Off, None));
        assert!(!scan_written(&q));
    }

    #[test]
    fn a_tf_card_on_the_wifi_host_stops_the_discard() {
        let m = Mock::new(true);
        let q = m.quiet(&good_rmmod(&m), false);
        let link = q.sys.join("block/mmcblk1/device");
        fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(q.sys.join("bus/mmc/devices/mmc2:0001"), &link).unwrap();
        let (wifi, card) = q.unload_wifi();
        assert_eq!(wifi, WifiResult::Off);
        assert!(matches!(card, Some(StaleCard::Left(_))), "{card:?}");
        assert!(!scan_written(&q));
        assert!(card_present(&q, "mmc2:0001"));
    }

    #[test]
    fn an_sdio_function_on_another_host_stops_the_discard() {
        let m = Mock::new(true);
        let q = m.quiet(&good_rmmod(&m), false);
        fs::create_dir_all(q.sys.join("bus/sdio/devices/mmc1:aaaa:1")).unwrap();
        let (_, card) = q.unload_wifi();
        assert!(matches!(card, Some(StaleCard::Left(_))), "{card:?}");
        assert!(!scan_written(&q));
    }

    #[test]
    fn no_device_tree_bus_number_stops_the_discard() {
        let m = Mock::new(true);
        let q = m.quiet(&good_rmmod(&m), false);
        fs::remove_dir_all(q.sys.join("firmware")).unwrap();
        let (_, card) = q.unload_wifi();
        assert!(matches!(card, Some(StaleCard::Left(_))), "{card:?}");
        assert!(!scan_written(&q));
    }

    #[test]
    fn a_rail_still_on_stops_the_discard() {
        let m = Mock::new(true);
        // This unload turns power_state off but leaves the WLAN rail enabled.
        let p = m.dir.path().display();
        let rmmod = format!(
            "#!/bin/sh\nrm -rf {p}/sys/class/net/wlan0\n\
             echo 0 > {p}/sys/class/misc/sunxi-wlan/rf-ctrl/power_state\n\
             echo 'mali_kbase 1 3 - Live 0x0' > {p}/proc/modules\n"
        );
        let q = m.quiet(&rmmod, false);
        let (wifi, card) = q.unload_wifi();
        assert_eq!(wifi, WifiResult::Off, "0.4.0's own check is unchanged");
        assert!(matches!(card, Some(StaleCard::Left(_))), "{card:?}");
        assert!(!scan_written(&q));
    }

    #[test]
    fn an_unreadable_rail_stops_the_discard() {
        let m = Mock::new(true);
        let q = m.quiet(&good_rmmod(&m), false);
        fs::write(
            q.sys.join("class/regulator/regulator.12/name"),
            "something-else\n",
        )
        .unwrap();
        let (_, card) = q.unload_wifi();
        assert!(matches!(card, Some(StaleCard::Left(_))), "{card:?}");
        assert!(!scan_written(&q));
    }

    #[test]
    fn a_missing_scan_switch_leaves_the_card() {
        let m = Mock::new(true);
        let q = m.quiet(&good_rmmod(&m), false);
        fs::remove_file(q.sys.join(SCAN_DEVICE)).unwrap();
        let (wifi, card) = q.unload_wifi();
        assert_eq!(wifi, WifiResult::Off);
        assert!(matches!(card, Some(StaleCard::Left(_))), "{card:?}");
        assert!(card_present(&q, "mmc2:0001"));
    }

    #[test]
    fn a_card_that_stays_is_reported_and_wifi_still_counts_as_off() {
        let m = Mock::new(true);
        let q = m.quiet(&good_rmmod(&m), false);
        let began = Instant::now();
        let (wifi, card) = q.unload_wifi();
        assert_eq!(wifi, WifiResult::Off);
        assert!(matches!(card, Some(StaleCard::Failed(_))), "{card:?}");
        assert!(
            began.elapsed() < Duration::from_secs(5),
            "the wait is bounded"
        );
        tf_cards_untouched(&q);
    }

    #[test]
    fn a_rescan_that_powers_the_radio_is_reported() {
        let m = Mock::new(true);
        let q = m.quiet(&good_rmmod(&m), false);
        let k = kernel(&m, true);
        let (_, card) = q.unload_wifi();
        k.join().unwrap();
        assert!(
            matches!(card, Some(StaleCard::Failed(ref w)) if w.contains("power_state")),
            "{card:?}"
        );
    }

    #[test]
    fn another_mmc_device_changing_is_reported() {
        let m = Mock::new(true);
        let q = m.quiet(&good_rmmod(&m), false);
        let sys = q.sys.clone();
        let k = std::thread::spawn(move || {
            let scan = sys.join(SCAN_DEVICE);
            while fs::read_to_string(&scan).unwrap_or_default().trim() != "1" {
                std::thread::sleep(Duration::from_millis(5));
            }
            fs::remove_file(sys.join("bus/mmc/devices/mmc2:0001")).unwrap();
            fs::remove_file(sys.join("bus/mmc/devices/mmc1:aaaa")).unwrap();
        });
        let (_, card) = q.unload_wifi();
        k.join().unwrap();
        assert!(
            matches!(card, Some(StaleCard::Failed(ref w)) if w.contains("another")),
            "{card:?}"
        );
    }
}
