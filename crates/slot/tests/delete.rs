mod common;

use common::{app_in_switcher, app_playing_in, tmp_root_with_carts};
use slot::app::Phase;
use slot_input::{Action, Btn};
use slot_store::{Core, Platform, StateRing};

/// Y asks, and a second Y deletes.
fn confirm_delete(a: &mut slot::app::App) {
    a.apply(Action::GbaDown(Btn::Y));
    a.apply(Action::GbaUp(Btn::Y));
    a.apply(Action::GbaDown(Btn::Y));
    a.apply(Action::GbaUp(Btn::Y));
}

#[test]
fn y_deletes_the_selected_state_and_nothing_else() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let r = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald");
    for i in 0..3 {
        r.push(&[i as u8; 64], b"png", &format!("2026-08-09_00-00-{i:02}"))
            .unwrap();
    }
    let mut a = app_in_switcher(d.path(), "Emerald");
    a.apply(Action::ShelfRight);
    confirm_delete(&mut a);
    let left = r.list().unwrap();
    assert_eq!(left.len(), 2, "delete removed the wrong number of states");
    assert!(
        !left.iter().any(|e| e.stamp == "2026-08-09_00-00-01"),
        "the wrong one went"
    );
}

#[test]
fn deleting_the_pending_undos_target_clears_the_offer() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply_at(Action::SaveState, 1_000);
    a.apply_at(Action::Polaroids, 1_100);
    assert!(a.undo_available(1_200));
    confirm_delete(&mut a);
    assert!(
        !a.undo_available(1_300),
        "the offer still points at a deleted state"
    );
}

#[test]
fn deleting_the_last_state_closes_the_switcher() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let r = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald");
    r.push(&[0u8; 64], b"png", "2026-08-09_00-00-00").unwrap();
    let mut a = app_in_switcher(d.path(), "Emerald");
    confirm_delete(&mut a);
    assert!(
        matches!(a.phase(), Phase::Playing { .. }),
        "an empty switcher stayed open"
    );
}

#[test]
fn the_deleted_state_leaves_the_switcher_with_it() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let r = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald");
    for i in 0..3 {
        r.push(&[i as u8; 64], b"png", &format!("2026-08-09_00-00-{i:02}"))
            .unwrap();
    }
    let mut a = app_in_switcher(d.path(), "Emerald");
    confirm_delete(&mut a);
    let stamps: Vec<&str> = a
        .polaroid_entries()
        .iter()
        .map(|e| e.stamp.as_str())
        .collect();
    assert_eq!(stamps, ["2026-08-09_00-00-01", "2026-08-09_00-00-00"]);
    assert!(
        matches!(a.phase(), Phase::Polaroids { .. }),
        "the switcher closed with states still in it"
    );
}

#[test]
fn deleting_someone_elses_state_leaves_the_offer_alone() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let r = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald");
    r.push(&[0u8; 64], b"png", "2026-08-09_00-00-00").unwrap();
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply_at(Action::SaveState, 1_000);
    a.apply_at(Action::Polaroids, 1_100);
    a.apply(Action::ShelfRight);
    confirm_delete(&mut a);
    assert!(
        a.undo_available(1_300),
        "an unrelated delete took the offer"
    );
}

fn three_states(d: &tempfile::TempDir) -> StateRing {
    let r = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald");
    for i in 0..3 {
        r.push(&[i as u8; 64], b"png", &format!("2026-08-09_00-00-{i:02}"))
            .unwrap();
    }
    r
}

#[test]
fn one_y_only_asks() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let r = three_states(&d);
    let mut a = app_in_switcher(d.path(), "Emerald");
    a.apply(Action::GbaDown(Btn::Y));
    assert_eq!(
        r.list().unwrap().len(),
        3,
        "the first Y deleted without asking"
    );
    assert!(a.confirming_delete());
    assert_eq!(
        a.polaroid_title("2026-08-09_00-00-05"),
        slot_ui::CONFIRM_TITLE,
        "the top plate does not ask"
    );
}

#[test]
fn b_keeps_the_state_and_stays_in_the_switcher() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let r = three_states(&d);
    let mut a = app_in_switcher(d.path(), "Emerald");
    a.apply(Action::GbaDown(Btn::Y));
    a.apply(Action::GbaDown(Btn::B));
    assert!(!a.confirming_delete());
    assert_eq!(r.list().unwrap().len(), 3);
    assert!(
        matches!(a.phase(), Phase::Polaroids { .. }),
        "B on the question closed the switcher instead of just keeping the state"
    );
}

#[test]
fn a_while_asking_neither_loads_nor_deletes() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let r = three_states(&d);
    let mut a = app_in_switcher(d.path(), "Emerald");
    a.apply(Action::GbaDown(Btn::Y));
    a.apply(Action::GbaDown(Btn::A));
    assert_eq!(r.list().unwrap().len(), 3);
    assert!(!a.confirming_delete());
    assert!(
        matches!(a.phase(), Phase::Polaroids { .. }),
        "A on the question loaded the state"
    );
    a.apply(Action::GbaDown(Btn::Y));
    assert_eq!(
        r.list().unwrap().len(),
        3,
        "the question did not start over"
    );
}

#[test]
fn up_and_down_move_through_the_states_newest_first() {
    let d = tmp_root_with_carts(&["Emerald"]);
    three_states(&d);
    let mut a = app_in_switcher(d.path(), "Emerald");
    assert_eq!(a.polaroid_stamp(), Some("2026-08-09_00-00-02"));
    a.apply(Action::GbaDown(Btn::Down));
    assert_eq!(a.polaroid_stamp(), Some("2026-08-09_00-00-01"));
    a.apply(Action::GbaDown(Btn::Down));
    a.apply(Action::GbaDown(Btn::Down));
    assert_eq!(
        a.polaroid_stamp(),
        Some("2026-08-09_00-00-00"),
        "Down ran off the end"
    );
    a.apply(Action::GbaDown(Btn::Up));
    assert_eq!(a.polaroid_stamp(), Some("2026-08-09_00-00-01"));
    assert!(
        matches!(a.phase(), Phase::Polaroids { .. }),
        "moving loaded or closed something"
    );
}
