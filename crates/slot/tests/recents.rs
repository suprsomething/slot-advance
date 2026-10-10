mod common;

use common::{app_booting_at, tmp_root_with_carts};
use slot::app::{App, Phase};
use slot_input::{Action, Btn};
use slot_store::{read_recents, write_slot_state, SlotState, RECENTS_FILE, RECENTS_MAX};
use tempfile::TempDir;

const CLOCK_IS_SET: i64 = 1_700_000_000;

fn carousel(stems: &[&str]) -> (TempDir, App) {
    let d = tmp_root_with_carts(stems);
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            ..Default::default()
        },
    )
    .expect("write slot.state");
    let (a, _clock) = app_booting_at(d.path(), CLOCK_IS_SET);
    assert!(matches!(a.phase(), Phase::Shelf), "{:?}", a.phase());
    (d, a)
}

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn r2(a: &mut App) {
    a.apply(Action::FfStart);
    a.apply(Action::FfStop);
}

fn l2(a: &mut App) {
    a.apply(Action::RewindStart);
    a.apply(Action::RewindStop);
}

fn settle(a: &mut App) {
    for _ in 0..120 {
        a.update(1.0 / 60.0);
    }
}

/// Starts the cart in front of the slot, plays it, and puts it back on the shelf.
fn play_and_eject(a: &mut App) -> String {
    let stem = a.selected_stem().expect("a cart").to_string();
    a.apply(Action::Insert);
    a.on_core_ready();
    settle(a);
    assert!(
        matches!(a.phase(), Phase::Playing { .. }),
        "{:?}",
        a.phase()
    );
    a.apply(Action::Eject);
    settle(a);
    assert!(matches!(a.phase(), Phase::Shelf), "{:?}", a.phase());
    stem
}

fn right(a: &mut App) {
    press(a, Btn::Right);
    a.update(1.0);
}

fn key(stem: &str) -> String {
    format!("GBA/{stem}")
}

#[test]
fn a_game_that_starts_goes_to_the_front_of_recents_and_onto_the_card() {
    let (d, mut a) = carousel(&["Advance Wars", "Emerald", "Fusion"]);
    assert!(a.recents().is_empty());
    let first = play_and_eject(&mut a);
    right(&mut a);
    let second = play_and_eject(&mut a);
    assert_eq!(a.recents(), &[key(&second), key(&first)]);
    assert_eq!(
        read_recents(d.path()),
        vec![key(&second), key(&first)],
        "Recents never reached {RECENTS_FILE}"
    );
}

#[test]
fn recents_sits_between_all_and_favourites_and_lists_newest_first() {
    let (_d, mut a) = carousel(&["Advance Wars", "Emerald", "Fusion"]);
    let first = play_and_eject(&mut a);
    right(&mut a);
    let second = play_and_eject(&mut a);
    press(&mut a, Btn::Y);

    r2(&mut a);
    assert!(a.on_recents(), "R2 from All did not open Recents");
    assert_eq!(a.shelf_platform_name(), Some("Recents"));
    assert_eq!(
        a.selected_stem(),
        Some(second.as_str()),
        "newest is not first"
    );
    right(&mut a);
    assert_eq!(a.selected_stem(), Some(first.as_str()));

    r2(&mut a);
    assert!(a.on_favourites(), "R2 from Recents did not open Favourites");
    r2(&mut a);
    assert!(
        !a.on_recents() && !a.on_favourites(),
        "R2 did not wrap to All"
    );
    l2(&mut a);
    assert!(
        a.on_favourites(),
        "L2 from All did not go back to Favourites"
    );
    l2(&mut a);
    assert!(a.on_recents());
}

#[test]
fn with_nothing_played_r2_skips_recents() {
    let (_d, mut a) = carousel(&["Advance Wars", "Emerald"]);
    press(&mut a, Btn::Y);
    r2(&mut a);
    assert!(a.on_favourites(), "an empty Recents was not skipped");
}

#[test]
fn replaying_a_recent_game_moves_it_to_the_front_and_keeps_it_selected() {
    let (_d, mut a) = carousel(&["Advance Wars", "Emerald", "Fusion"]);
    let first = play_and_eject(&mut a);
    right(&mut a);
    let second = play_and_eject(&mut a);
    r2(&mut a);
    right(&mut a);
    assert_eq!(a.selected_stem(), Some(first.as_str()));
    play_and_eject(&mut a);
    assert!(a.on_recents());
    assert_eq!(a.recents(), &[key(&first), key(&second)]);
    assert_eq!(
        a.selected_stem(),
        Some(first.as_str()),
        "the cart just played is not the one in front of the slot"
    );
}

#[test]
fn recents_keeps_ten_and_survives_a_reboot() {
    let stems: Vec<String> = (0..12).map(|i| format!("Game {i:02}")).collect();
    let refs: Vec<&str> = stems.iter().map(String::as_str).collect();
    let (d, mut a) = carousel(&refs);
    for _ in 0..12 {
        play_and_eject(&mut a);
        right(&mut a);
    }
    assert_eq!(a.recents().len(), RECENTS_MAX);
    let newest = a.recents()[0].clone();
    drop(a);
    let (mut b, _clock) = app_booting_at(d.path(), CLOCK_IS_SET);
    assert_eq!(b.recents().len(), RECENTS_MAX);
    r2(&mut b);
    assert!(b.on_recents());
    assert_eq!(
        b.selected_stem().map(key),
        Some(newest),
        "the newest recent is not first after a reboot"
    );
}

#[test]
fn a_recents_file_edited_by_hand_is_read_and_junk_is_ignored() {
    let (d, _a) = carousel(&["Emerald", "Fusion"]);
    std::fs::write(
        d.path().join(RECENTS_FILE),
        "GBA/Fusion\n\nnot a cart\nGBA/\r\nGBA/Fusion\nGBA/Emerald\r\nGBA/Gone\n",
    )
    .unwrap();
    assert_eq!(
        read_recents(d.path()),
        vec![key("Fusion"), key("Emerald"), key("Gone")],
        "duplicates or junk were kept, or a missing game lost its place"
    );
    let (b, _clock) = app_booting_at(d.path(), CLOCK_IS_SET);
    let mut b = b;
    r2(&mut b);
    assert!(b.on_recents());
    assert_eq!(b.selected_stem(), Some("Fusion"));
    right(&mut b);
    assert_eq!(b.selected_stem(), Some("Emerald"));
}
