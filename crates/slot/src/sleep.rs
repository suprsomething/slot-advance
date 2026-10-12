//! Hardware sleep: what a doze does once the game is safely saved.
//!
//! Closing the lid (or a tap of POWER) saves the game first, exactly as before. Then,
//! where the platform supports it, the device suspends to RAM with the lid and a timer
//! (rtc1) as wake sources. Every wake is classified the same way:
//!
//! 1. the lid is open: carry on playing, straight away, with nothing reloaded;
//! 2. the sleep duration is up: shut down cleanly (the next boot resumes the game);
//! 3. anything else (POWER with the lid shut, a charger, a stray event): back to sleep,
//!    screen dark, with the same deadline.
//!
//! Hardware sleep that fails, or a run of wakes that never settles, falls back to the
//! software standby of v3 for the rest of that doze. A resume state that could not be
//! written is never thrown away by an automatic shutdown: the shutdown waits and the write
//! is retried.
//!
//! The plan here is pure bookkeeping, so the decisions can be tested on any machine.

use slot_input::Millis;
use slot_power::{Battery, Charge};
pub use slot_store::{SLEEP_MINUTES, SLEEP_MINUTES_DEFAULT};

/// A pause after the lid closes before the first suspend, so a quick flick of the lid
/// (seen on the next 150 ms lid poll) never reaches the kernel.
pub const SETTLE_MS: Millis = 400;

/// A pause after a wake that did not open the lid, before sleeping again.
pub const REST_AFTER_WAKE_MS: Millis = 1_000;

/// POWER is ignored this long after any hardware wake, so the press that woke the device
/// (which reaches slot just after resume) cannot also put it back to sleep or open a menu.
pub const POWER_QUIET_MS: Millis = 3_000;

/// More unexpected wakes than this in one doze is a wake storm: give up on hardware sleep
/// for the rest of the doze.
pub const MAX_WAKES: u32 = 20;

/// Consecutive suspends that fail before falling back (one retry).
pub const MAX_FAILURES: u32 = 2;

/// Wakes this close to the deadline count as the deadline.
pub const DEADLINE_SLACK_S: f64 = 5.0;

/// Sleep is planned to leave at least this much battery.
pub const BATTERY_FLOOR: u8 = 10;

/// Seconds of sleep one battery percent is assumed to buy: 35 mAh at a deliberately
/// pessimistic 100 mA (measured: about 70 mA, up to about 98 mA with the USB controllers
/// on).
pub const SECS_PER_PERCENT: f64 = 35.0 / 100.0 * 3600.0;

/// Even at or below the floor, a doze may last this long, as v3's standby did.
pub const LOW_BATTERY_SLEEP_S: f64 = 180.0;

/// When the deadline arrives but the game could not be saved, sleep this much longer and
/// try the save again rather than shut down.
pub const UNSAVED_RETRY_S: f64 = 1_800.0;

#[derive(Clone, Debug, PartialEq)]
pub struct SleepPlan {
    /// Uptime (which counts time suspended) at which this doze ends in a shutdown.
    pub deadline: Option<f64>,
    /// No suspend before this app time.
    pub next_try: Millis,
    pub wakes: u32,
    pub failures: u32,
    /// Software standby for the rest of this doze.
    pub fallback: bool,
    /// The resume state could not be written at lid close.
    pub unsaved: bool,
}

impl Default for SleepPlan {
    fn default() -> Self {
        SleepPlan {
            deadline: None,
            next_try: 0,
            wakes: 0,
            failures: 0,
            fallback: true,
            unsaved: false,
        }
    }
}

/// How a wake should be handled.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The lid is open: carry on playing.
    Resume,
    /// The sleep duration is up.
    Expire,
    /// Back to sleep after a short rest.
    SleepAgain,
    /// The suspend did not happen; try once more after a short rest.
    Retry,
    /// Give up on hardware sleep for this doze.
    Fallback,
}

impl SleepPlan {
    /// A fresh plan for a doze that began now. `uptime` is `None` where the platform has
    /// no hardware sleep, which leaves the doze as software standby.
    pub fn begin(now: Millis, uptime: Option<f64>, minutes: u16, saved: bool) -> SleepPlan {
        match uptime {
            Some(up) => SleepPlan {
                deadline: Some(up + f64::from(minutes) * 60.0),
                next_try: now + SETTLE_MS,
                wakes: 0,
                failures: 0,
                fallback: false,
                unsaved: !saved,
            },
            None => SleepPlan {
                unsaved: !saved,
                ..SleepPlan::default()
            },
        }
    }

    /// Brings the deadline forward if the battery could fall below the floor first.
    /// Charging lifts the cap. Returns the seconds left until the deadline.
    pub fn cap_for_battery(&mut self, uptime: f64, battery: Option<Battery>) -> Option<f64> {
        let deadline = self.deadline?;
        if let Some(b) = battery {
            if !matches!(b.charge, Charge::Charging | Charge::Full) {
                let allowance = if b.percent <= BATTERY_FLOOR {
                    LOW_BATTERY_SLEEP_S
                } else {
                    f64::from(b.percent - BATTERY_FLOOR) * SECS_PER_PERCENT
                };
                let capped = uptime + allowance.max(LOW_BATTERY_SLEEP_S);
                if capped < deadline {
                    self.deadline = Some(capped);
                }
            }
        }
        self.deadline.map(|d| d - uptime)
    }

    pub fn due(&self, uptime: f64) -> bool {
        self.deadline
            .is_some_and(|d| uptime >= d - DEADLINE_SLACK_S)
    }

    /// Classifies a wake. `lid_open` is read from the hardware after resume; `slept` is
    /// whether the kernel really suspended.
    pub fn judge(
        &mut self,
        now: Millis,
        lid_open: Option<bool>,
        slept: bool,
        uptime: f64,
    ) -> Verdict {
        if lid_open == Some(true) {
            return Verdict::Resume;
        }
        if !slept {
            self.failures += 1;
            if self.failures >= MAX_FAILURES {
                self.fallback = true;
                return Verdict::Fallback;
            }
            self.next_try = now + REST_AFTER_WAKE_MS;
            return Verdict::Retry;
        }
        self.failures = 0;
        if self.due(uptime) {
            return Verdict::Expire;
        }
        self.wakes += 1;
        if self.wakes > MAX_WAKES {
            self.fallback = true;
            return Verdict::Fallback;
        }
        self.next_try = now + REST_AFTER_WAKE_MS;
        Verdict::SleepAgain
    }

    /// The deadline came but the game is not saved: keep it asleep a while longer.
    pub fn postpone(&mut self, now: Millis, uptime: f64) {
        self.deadline = Some(uptime + UNSAVED_RETRY_S);
        self.next_try = now + REST_AFTER_WAKE_MS;
    }
}

/// The next sleep duration along the list, stopping at the ends.
pub fn sleep_step(from: u16, right: bool) -> u16 {
    let at = SLEEP_MINUTES
        .iter()
        .position(|&m| m == from)
        .unwrap_or_else(|| {
            SLEEP_MINUTES
                .iter()
                .position(|&m| m == SLEEP_MINUTES_DEFAULT)
                .unwrap_or(0)
        });
    let to = if right {
        (at + 1).min(SLEEP_MINUTES.len() - 1)
    } else {
        at.saturating_sub(1)
    };
    SLEEP_MINUTES[to]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draining(percent: u8) -> Option<Battery> {
        Some(Battery {
            percent,
            charge: Charge::Discharging,
        })
    }

    #[test]
    fn the_deadline_is_the_whole_doze_from_lid_close() {
        let p = SleepPlan::begin(1_000, Some(500.0), 30, true);
        assert_eq!(p.deadline, Some(500.0 + 1800.0));
        assert_eq!(p.next_try, 1_000 + SETTLE_MS);
        assert!(!p.fallback);
    }

    #[test]
    fn without_hardware_sleep_the_doze_is_software_standby() {
        let p = SleepPlan::begin(0, None, 30, true);
        assert!(p.fallback);
        assert_eq!(p.deadline, None);
    }

    #[test]
    fn a_full_battery_does_not_shorten_the_sleep() {
        let mut p = SleepPlan::begin(0, Some(0.0), 480, true);
        let left = p.cap_for_battery(0.0, draining(90)).unwrap();
        assert_eq!(left, 480.0 * 60.0);
    }

    #[test]
    fn a_low_battery_wakes_in_time_to_leave_ten_percent() {
        let mut p = SleepPlan::begin(0, Some(0.0), 480, true);
        let left = p.cap_for_battery(0.0, draining(20)).unwrap();
        assert_eq!(left, 10.0 * SECS_PER_PERCENT, "10 points at 100 mA");
        assert!(left < 480.0 * 60.0);
    }

    #[test]
    fn at_or_below_the_floor_it_still_sleeps_three_minutes() {
        for pct in [10, 8, 6] {
            let mut p = SleepPlan::begin(0, Some(0.0), 480, true);
            assert_eq!(
                p.cap_for_battery(0.0, draining(pct)),
                Some(LOW_BATTERY_SLEEP_S)
            );
        }
    }

    #[test]
    fn charging_lifts_the_cap() {
        let mut p = SleepPlan::begin(0, Some(0.0), 480, true);
        let charging = Some(Battery {
            percent: 12,
            charge: Charge::Charging,
        });
        assert_eq!(p.cap_for_battery(0.0, charging), Some(480.0 * 60.0));
    }

    #[test]
    fn an_open_lid_always_resumes_even_after_a_failed_suspend() {
        let mut p = SleepPlan::begin(0, Some(0.0), 30, true);
        assert_eq!(p.judge(500, Some(true), false, 10.0), Verdict::Resume);
        assert_eq!(p.judge(500, Some(true), true, 1_799.0), Verdict::Resume);
    }

    #[test]
    fn the_alarm_with_the_lid_shut_is_the_end() {
        let mut p = SleepPlan::begin(0, Some(0.0), 30, true);
        assert_eq!(p.judge(500, Some(false), true, 1_799.0), Verdict::Expire);
    }

    #[test]
    fn power_or_a_stray_wake_with_the_lid_shut_sleeps_again() {
        let mut p = SleepPlan::begin(0, Some(0.0), 30, true);
        assert_eq!(p.judge(500, Some(false), true, 600.0), Verdict::SleepAgain);
        assert_eq!(p.next_try, 500 + REST_AFTER_WAKE_MS);
        assert_eq!(p.deadline, Some(1_800.0), "the deadline did not move");
    }

    #[test]
    fn a_failed_suspend_is_retried_once_then_falls_back() {
        let mut p = SleepPlan::begin(0, Some(0.0), 30, true);
        assert_eq!(p.judge(500, Some(false), false, 1.0), Verdict::Retry);
        assert_eq!(p.judge(1_600, Some(false), false, 2.0), Verdict::Fallback);
        assert!(p.fallback);
    }

    #[test]
    fn a_wake_storm_falls_back() {
        let mut p = SleepPlan::begin(0, Some(0.0), 480, true);
        for i in 0..MAX_WAKES {
            assert_eq!(
                p.judge(i as Millis, Some(false), true, f64::from(i)),
                Verdict::SleepAgain
            );
        }
        assert_eq!(p.judge(99, Some(false), true, 99.0), Verdict::Fallback);
    }

    #[test]
    fn the_steps_stop_at_the_ends() {
        assert_eq!(sleep_step(30, true), 60);
        assert_eq!(sleep_step(30, false), 15);
        assert_eq!(sleep_step(3, false), 3);
        assert_eq!(sleep_step(480, true), 480);
        assert_eq!(
            sleep_step(7, true),
            60,
            "an odd value starts from the default"
        );
    }
}
