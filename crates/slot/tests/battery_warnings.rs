//! Quiet banners at 15% and 10%, once per discharge, before the existing 5% shutdown.

mod common;

use common::{app_playing_in, tmp_root_with_carts};
use slot::app::App;
use slot_input::Action;
use slot_power::{Battery, Charge};
use slot_ui::Toast;

fn draining(percent: u8) -> Battery {
    Battery {
        percent,
        charge: Charge::Discharging,
    }
}

fn charging(percent: u8) -> Battery {
    Battery {
        percent,
        charge: Charge::Charging,
    }
}

fn playing() -> (tempfile::TempDir, App) {
    let d = tmp_root_with_carts(&["Emerald"]);
    let a = app_playing_in(d.path(), "Emerald");
    (d, a)
}

/// Lets any banner up now fade, so the next one can be told apart.
fn wait_out(a: &mut App, now: &mut u64) {
    *now += 10_000;
    a.tick_ms(*now);
}

#[test]
fn nothing_is_said_above_fifteen_percent() {
    let (_d, mut a) = playing();
    for p in [100, 50, 16] {
        a.on_battery(draining(p));
        assert_eq!(a.toast(), None, "a banner at {p}%");
    }
}

#[test]
fn fifteen_then_ten_each_once() {
    let (_d, mut a) = playing();
    let mut now = 1_000;
    a.on_battery(draining(15));
    assert_eq!(a.toast(), Some(Toast::BatteryLow15));
    wait_out(&mut a, &mut now);
    for p in [14, 13, 12, 11] {
        a.on_battery(draining(p));
        assert_eq!(a.toast(), None, "the 15% banner repeated at {p}%");
    }
    a.on_battery(draining(10));
    assert_eq!(a.toast(), Some(Toast::BatteryLow10));
    wait_out(&mut a, &mut now);
    for p in [9, 8, 7, 6] {
        a.on_battery(draining(p));
        assert_eq!(a.toast(), None, "the 10% banner repeated at {p}%");
    }
    assert!(!a.shutting_down(), "a warning level shut the device down");
}

#[test]
fn a_reading_past_both_levels_shows_only_the_lower() {
    let (_d, mut a) = playing();
    a.on_battery(draining(9));
    assert_eq!(a.toast(), Some(Toast::BatteryLow10));
}

#[test]
fn no_banner_while_charging_and_charging_rearms_them() {
    let (_d, mut a) = playing();
    let mut now = 1_000;
    a.on_battery(charging(12));
    assert_eq!(a.toast(), None, "a low banner while charging");
    a.on_battery(draining(12));
    assert_eq!(a.toast(), Some(Toast::BatteryLow15));
    wait_out(&mut a, &mut now);
    a.on_battery(charging(40));
    a.on_battery(draining(15));
    assert_eq!(
        a.toast(),
        Some(Toast::BatteryLow15),
        "charging did not re-arm the warning"
    );
}

#[test]
fn no_banner_while_dozing_but_one_after_waking() {
    let (_d, mut a) = playing();
    a.apply(Action::LidClose);
    a.on_battery(draining(14));
    a.apply(Action::LidOpen);
    assert_eq!(a.toast(), None, "the banner was raised behind a shut lid");
    a.on_battery(draining(14));
    assert_eq!(a.toast(), Some(Toast::BatteryLow15));
}

#[test]
fn five_percent_still_shuts_down() {
    let (_d, mut a) = playing();
    a.on_battery(draining(15));
    a.on_battery(draining(10));
    a.on_battery(draining(5));
    assert!(a.shutting_down(), "the critical shutdown was lost");
}

#[test]
fn the_banners_show_on_the_carousel_too() {
    let (_d, mut a) = playing();
    a.apply(Action::Eject);
    for _ in 0..120 {
        a.update(1.0 / 60.0);
    }
    a.on_battery(draining(15));
    assert_eq!(a.toast(), Some(Toast::BatteryLow15));
}
