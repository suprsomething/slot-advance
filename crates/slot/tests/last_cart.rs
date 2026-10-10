//! After a reboot with the slot empty, the carousel opens on the game played last, in the
//! category (All, Recents or Favourites) it was started from.

mod common;

use common::{app_booting_at, tmp_root_with_carts};
use slot::app::{App, Phase};
use slot_input::{Action, Btn};
use slot_store::{read_slot_state, write_slot_state, Category, SlotState};
use tempfile::TempDir;

const CLOCK_IS_SET: i64 = 1_700_000_000;
const CARTS: [&str; 4] = ["Advance Wars", "Emerald", "Fusion", "Minish Cap"];

fn card() -> TempDir {
    let d = tmp_root_with_carts(&CARTS);
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            ..Default::default()
        },
    )
    .expect("write slot.state");
    d
}

fn boot(d: &TempDir) -> App {
    let (a, _clock) = app_booting_at(d.path(), CLOCK_IS_SET);
    a
}

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn r2(a: &mut App) {
    a.apply(Action::FfStart);
    a.apply(Action::FfStop);
}

fn settle(a: &mut App) {
    for _ in 0..120 {
        a.update(1.0 / 60.0);
    }
}

fn select(a: &mut App, stem: &str) {
    for _ in 0..8 {
        if a.selected_stem() == Some(stem) {
            return;
        }
        press(a, Btn::Right);
        a.update(1.0);
    }
    for _ in 0..8 {
        if a.selected_stem() == Some(stem) {
            return;
        }
        press(a, Btn::Left);
        a.update(1.0);
    }
    panic!("{stem} is not on this shelf: {:?}", a.selected_stem());
}

fn start(a: &mut App) {
    a.apply(Action::Insert);
    a.on_core_ready();
    settle(a);
    assert!(
        matches!(a.phase(), Phase::Playing { .. }),
        "{:?}",
        a.phase()
    );
}

fn eject(a: &mut App) {
    a.apply(Action::Eject);
    settle(a);
    assert!(matches!(a.phase(), Phase::Shelf), "{:?}", a.phase());
}

fn play_and_eject(a: &mut App, stem: &str) {
    select(a, stem);
    start(a);
    eject(a);
}

fn open_recents(a: &mut App) {
    for _ in 0..3 {
        if a.on_recents() {
            return;
        }
        r2(a);
    }
    assert!(a.on_recents(), "R2 never reached Recents");
}

fn open_favourites(a: &mut App) {
    for _ in 0..3 {
        if a.on_favourites() {
            return;
        }
        r2(a);
    }
    assert!(a.on_favourites(), "R2 never reached Favourites");
}

fn on_all(a: &App) -> bool {
    !a.on_recents() && !a.on_favourites()
}

#[test]
fn a_game_started_from_all_is_in_front_after_a_reboot() {
    let d = card();
    let mut a = boot(&d);
    play_and_eject(&mut a, "Fusion");
    drop(a);

    let a = boot(&d);
    assert!(matches!(a.phase(), Phase::Shelf), "{:?}", a.phase());
    assert!(on_all(&a), "opened on a category instead of All");
    assert_eq!(a.selected_stem(), Some("Fusion"));
    assert_eq!(read_slot_state(d.path()).last_category, Category::All);
}

#[test]
fn a_game_started_from_recents_reopens_recents() {
    let d = card();
    let mut a = boot(&d);
    play_and_eject(&mut a, "Emerald");
    play_and_eject(&mut a, "Minish Cap");
    open_recents(&mut a);
    play_and_eject(&mut a, "Emerald");
    drop(a);

    let a = boot(&d);
    assert!(a.on_recents(), "did not reopen Recents");
    assert_eq!(a.selected_stem(), Some("Emerald"));
    assert_eq!(read_slot_state(d.path()).last_category, Category::Recents);
}

#[test]
fn a_game_started_from_favourites_reopens_favourites() {
    let d = card();
    let mut a = boot(&d);
    select(&mut a, "Advance Wars");
    press(&mut a, Btn::Y);
    select(&mut a, "Minish Cap");
    press(&mut a, Btn::Y);
    open_favourites(&mut a);
    play_and_eject(&mut a, "Minish Cap");
    drop(a);

    let a = boot(&d);
    assert!(a.on_favourites(), "did not reopen Favourites");
    assert_eq!(a.selected_stem(), Some("Minish Cap"));
}

#[test]
fn an_unstarred_game_falls_back_to_all() {
    let d = card();
    let mut a = boot(&d);
    select(&mut a, "Fusion");
    press(&mut a, Btn::Y);
    open_favourites(&mut a);
    play_and_eject(&mut a, "Fusion");
    // Unstarring it empties Favourites, which sends the carousel back to the library.
    press(&mut a, Btn::Y);
    drop(a);

    let a = boot(&d);
    assert!(on_all(&a), "opened an empty or stale category");
    assert_eq!(a.selected_stem(), Some("Fusion"));
}

#[test]
fn a_game_taken_off_the_card_leaves_the_carousel_on_its_first_cart() {
    let d = card();
    let mut a = boot(&d);
    play_and_eject(&mut a, "Fusion");
    drop(a);
    std::fs::remove_file(d.path().join("Games/GBA/Fusion.gba")).expect("remove the rom");

    let a = boot(&d);
    assert!(matches!(a.phase(), Phase::Shelf), "{:?}", a.phase());
    assert!(on_all(&a));
    assert_eq!(a.selected_stem(), Some("Advance Wars"));
}

#[test]
fn resuming_the_seated_game_at_boot_keeps_the_remembered_category() {
    let d = card();
    let mut a = boot(&d);
    play_and_eject(&mut a, "Emerald");
    open_recents(&mut a);
    select(&mut a, "Emerald");
    start(&mut a);
    // Power off with the game still in the slot: the next boot resumes it from the library.
    a.flush_state();
    drop(a);

    let mut a = boot(&d);
    assert!(
        matches!(a.phase(), Phase::Inserting { .. }),
        "{:?}",
        a.phase()
    );
    a.on_core_ready();
    settle(&mut a);
    assert!(matches!(a.phase(), Phase::Playing { .. }));
    assert_eq!(
        read_slot_state(d.path()).last_category,
        Category::Recents,
        "the boot-time resume overwrote where the game was started from"
    );
    eject(&mut a);
    drop(a);

    let a = boot(&d);
    assert!(a.on_recents());
    assert_eq!(a.selected_stem(), Some("Emerald"));
}

#[test]
fn a_fresh_card_with_no_history_opens_on_the_first_cart() {
    let d = card();
    let a = boot(&d);
    assert!(on_all(&a));
    assert_eq!(a.selected_stem(), Some("Advance Wars"));
}

#[test]
fn an_older_slot_state_without_the_line_reads_as_all() {
    let d = card();
    let path = d.path().join("Config/slot.state");
    let text = std::fs::read_to_string(&path).unwrap();
    let older: String = text
        .lines()
        .filter(|l| !l.starts_with("last_category="))
        .map(|l| format!("{l}\n"))
        .collect();
    std::fs::write(&path, older).unwrap();
    let s = read_slot_state(d.path());
    assert!(s.clock_set, "the rest of the file was lost");
    assert_eq!(s.last_category, Category::All);
}
