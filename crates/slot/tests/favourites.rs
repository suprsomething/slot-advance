mod common;

use common::{app_booting_at, tmp_root_with_carts};
use slot::app::{App, Phase};
use slot_input::{Action, Btn};
use slot_store::{read_favourites, write_slot_state, SlotState, FAVOURITES_FILE};
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

fn star(a: &mut App) {
    press(a, Btn::Y);
}

fn key(stem: &str) -> String {
    format!("GBA/{stem}")
}

#[test]
fn y_stars_the_cart_in_front_and_writes_it_to_the_card() {
    let (d, mut a) = carousel(&["Advance Wars", "Emerald", "Fusion"]);
    let first = a.selected_stem().expect("a cart").to_string();
    star(&mut a);
    assert!(a.favourites().contains(&key(&first)));
    assert_eq!(
        read_favourites(d.path()),
        [key(&first)].into_iter().collect(),
        "the star never reached {FAVOURITES_FILE}"
    );
    star(&mut a);
    assert!(a.favourites().is_empty(), "a second Y did not unstar it");
    assert!(read_favourites(d.path()).is_empty());
}

#[test]
fn with_nothing_starred_l2_and_r2_stay_on_all_and_shake() {
    let (_d, mut a) = carousel(&["Emerald", "Fusion"]);
    r2(&mut a);
    assert!(!a.on_favourites());
    assert!(
        a.refusal_active(a.now()),
        "R2 with no favourites gave no feedback"
    );
    l2(&mut a);
    assert!(!a.on_favourites());
}

#[test]
fn r2_and_l2_step_between_all_and_favourites() {
    let (_d, mut a) = carousel(&["Advance Wars", "Emerald", "Fusion"]);
    press(&mut a, Btn::Right);
    a.update(1.0);
    let starred = a.selected_stem().expect("a cart").to_string();
    star(&mut a);

    r2(&mut a);
    assert!(a.on_favourites(), "R2 did not open Favourites");
    assert_eq!(a.shelf_platform_name(), Some("Favourites"));
    assert_eq!(a.selected_stem(), Some(starred.as_str()));
    assert_eq!(
        a.carts().count(),
        3,
        "the Favourites shelf leaked into the library count"
    );

    r2(&mut a);
    assert!(!a.on_favourites(), "R2 did not wrap back to All");
    assert_eq!(a.shelf_platform_name(), Some("All"));

    l2(&mut a);
    assert!(a.on_favourites(), "L2 did not step back to Favourites");
    l2(&mut a);
    assert!(!a.on_favourites());
}

#[test]
fn favourites_survive_a_reboot() {
    let (d, mut a) = carousel(&["Advance Wars", "Emerald", "Fusion"]);
    star(&mut a);
    let starred = a.selected_stem().expect("a cart").to_string();
    drop(a);
    let (b, _clock) = app_booting_at(d.path(), CLOCK_IS_SET);
    assert!(b.favourites().contains(&key(&starred)));
    let mut b = b;
    r2(&mut b);
    assert!(
        b.on_favourites(),
        "the starred cart did not come back as a favourite"
    );
    assert_eq!(b.selected_stem(), Some(starred.as_str()));
}

#[test]
fn unstarring_inside_favourites_slides_the_next_one_in_and_leaves_when_empty() {
    let (_d, mut a) = carousel(&["Advance Wars", "Emerald", "Fusion"]);
    star(&mut a);
    press(&mut a, Btn::Right);
    a.update(1.0);
    star(&mut a);
    r2(&mut a);
    assert!(a.on_favourites());
    let first = a.selected_stem().expect("a cart").to_string();

    star(&mut a);
    assert!(
        a.on_favourites(),
        "one favourite is left, so this is still Favourites"
    );
    let next = a.selected_stem().expect("the other favourite").to_string();
    assert_ne!(first, next);

    star(&mut a);
    assert!(
        !a.on_favourites(),
        "the last favourite went and the carousel stayed on an empty shelf"
    );
    assert!(a.selected_stem().is_some(), "All has nothing selected");
}

#[test]
fn a_favourites_file_edited_by_hand_is_read_and_junk_is_ignored() {
    let (d, _a) = carousel(&["Emerald", "Fusion"]);
    std::fs::write(
        d.path().join(FAVOURITES_FILE),
        "GBA/Fusion\n\nnot a cart\nN64/Mario\nGBA/\r\nGBA/Emerald\r\n",
    )
    .unwrap();
    let got = read_favourites(d.path());
    assert_eq!(got, [key("Emerald"), key("Fusion")].into_iter().collect());
}

#[test]
fn a_cart_can_start_from_favourites() {
    let (_d, mut a) = carousel(&["Advance Wars", "Emerald"]);
    press(&mut a, Btn::Right);
    a.update(1.0);
    let starred = a.selected_stem().expect("a cart").to_string();
    star(&mut a);
    r2(&mut a);
    a.apply(Action::Insert);
    match a.phase() {
        Phase::Inserting { cart, .. } => assert_eq!(cart, &starred),
        other => panic!("A on Favourites did not insert: {other:?}"),
    }
}
