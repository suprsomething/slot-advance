mod common;

use common::{app_playing_in, app_playing_with, tmp_root_with_carts, CoreSnapshot};
use slot_input::{Action, Btn};
use slot_store::{Core, Platform, StateRing, RING_MAX};

#[test]
fn undoing_a_save_removes_it_and_restores_the_evicted_entry() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let r = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald");
    for i in 0..RING_MAX {
        r.push(&[i as u8; 64], b"png", &format!("2026-08-09_00-00-{i:02}"))
            .unwrap();
    }
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply_at(Action::SaveState, 1_000);
    assert_eq!(r.list().unwrap().len(), RING_MAX);
    assert!(r
        .list()
        .unwrap()
        .iter()
        .all(|e| e.stamp != "2026-08-09_00-00-00"));

    a.undo(2_000);
    let l = r.list().unwrap();
    assert_eq!(l.len(), RING_MAX);
    assert!(
        l.iter().any(|e| e.stamp == "2026-08-09_00-00-00"),
        "the evicted oldest entry was not restored"
    );
}

fn future_ring(root: &std::path::Path, count: usize) -> StateRing {
    let r = StateRing::new(root, Platform::Gba, Core::Mgba, "Emerald");
    for i in 0..count {
        r.push(&[i as u8; 64], b"png", &format!("2099-01-01_00-00-{i:02}"))
            .unwrap();
    }
    r
}

fn stamps(r: &StateRing) -> Vec<String> {
    r.list().unwrap().into_iter().map(|e| e.stamp).collect()
}

#[test]
fn a_save_behind_the_newest_stamp_survives_a_full_ring() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let r = future_ring(d.path(), RING_MAX);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply_at(Action::SaveState, 1_000);
    let l = stamps(&r);
    assert_eq!(l.len(), RING_MAX);
    assert_eq!(
        l[0],
        format!("2099-01-01_00-00-{RING_MAX:02}"),
        "the new save was not stamped after the newest one (it was evicted by its own push)"
    );
    assert!(
        l.iter().all(|s| s != "2099-01-01_00-00-00"),
        "the oldest entry was kept instead of making room"
    );
}

#[test]
fn a_save_behind_the_newest_stamp_sorts_first_in_a_part_full_ring() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let r = future_ring(d.path(), 2);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply_at(Action::SaveState, 1_000);
    assert_eq!(
        stamps(&r),
        [
            "2099-01-01_00-00-02",
            "2099-01-01_00-00-01",
            "2099-01-01_00-00-00"
        ],
        "the new save did not land at the top of the switcher"
    );
}

#[test]
fn undoing_a_save_behind_the_newest_stamp_restores_the_full_ring() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let r = future_ring(d.path(), RING_MAX);
    let before = stamps(&r);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply_at(Action::SaveState, 1_000);
    assert_ne!(stamps(&r), before, "the save changed nothing");
    a.undo(2_000);
    assert_eq!(stamps(&r), before, "undo did not put the ring back");
    let (state, _) = r
        .read("2099-01-01_00-00-00")
        .expect("the evicted state is back");
    assert_eq!(state, vec![0u8; 64], "the restored state lost its bytes");
}

#[test]
fn undo_expires_after_thirty_seconds() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply_at(Action::SaveState, 1_000);
    assert!(a.undo_available(30_999));
    assert!(!a.undo_available(31_001));
}

#[test]
fn a_second_save_replaces_the_undo_rather_than_stacking() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let r = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald");
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply_at(Action::SaveState, 1_000);
    a.apply_at(Action::SaveState, 2_000);
    a.undo(3_000);
    assert_eq!(
        r.list().unwrap().len(),
        1,
        "only the most recent save should be undone"
    );
}

#[test]
fn undoing_a_load_returns_the_emulator_to_the_prior_state() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let core = CoreSnapshot::new();
    let mut a = app_playing_with(d.path(), "Emerald", core.boxed());
    core.run_frame();
    a.apply_at(Action::SaveState, 500);
    for _ in 0..120 {
        core.run_frame();
    }
    let before = core.bytes();
    a.apply_at(Action::LoadState, 1_000);
    assert_ne!(core.bytes(), before, "the load never moved the core");
    a.undo(2_000);
    assert_eq!(core.bytes(), before);
}

#[test]
fn undo_is_not_offered_when_nothing_is_undoable() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let a = app_playing_in(d.path(), "Emerald");
    assert!(!a.undo_available(0));
}

#[test]
fn the_label_names_what_will_be_undone() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply_at(Action::SaveState, 1_000);
    assert_eq!(a.undo_label(), Some("undo save"));
    a.apply_at(Action::LoadState, 2_000);
    assert_eq!(a.undo_label(), Some("undo load"));
}

#[test]
fn x_undoes_from_the_switcher_and_never_from_the_game() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let r = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald");
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply_at(Action::SaveState, 1_000);
    a.apply_at(Action::GbaDown(Btn::X), 1_100);
    assert_eq!(r.list().unwrap().len(), 1, "X undid from the game");

    a.apply_at(Action::Polaroids, 1_200);
    a.apply_at(Action::GbaDown(Btn::X), 1_300);
    assert!(r.list().unwrap().is_empty(), "X did not undo the save");
}

#[test]
fn undoing_twice_does_not_put_the_save_back() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let r = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald");
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply_at(Action::SaveState, 1_000);
    a.undo(2_000);
    a.undo(3_000);
    assert!(r.list().unwrap().is_empty());
    assert!(!a.undo_available(3_000));
}

#[test]
fn ejecting_the_cart_takes_its_undo_with_it() {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply_at(Action::SaveState, 1_000);
    a.apply_at(Action::Eject, 1_100);
    assert!(!a.undo_available(1_200));
}

#[test]
fn the_plate_is_drawn_only_over_the_switcher_and_only_while_the_offer_stands() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply_at(Action::SaveState, 1_000);
    a.tick_ms(4_000);
    let mut playing = Vec::new();
    a.draw(&mut playing);
    assert!(
        a.undo_available(4_000),
        "the offer expired before it was tested"
    );
    assert!(playing.is_empty(), "the offer reached the game");

    a.apply_at(Action::Polaroids, 4_000);
    let mut open = Vec::new();
    a.draw(&mut open);
    a.tick_ms(40_000);
    let mut expired = Vec::new();
    a.draw(&mut expired);
    assert_eq!(
        open.len(),
        expired.len() + 1,
        "the plate outlived the offer"
    );
}

#[test]
fn undoing_the_save_that_moved_old_states_aside_puts_nothing_back_twice() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let dir = d.path().join("States/GBA/mgba/Emerald");
    std::fs::create_dir_all(&dir).unwrap();
    for i in 0..10 {
        std::fs::write(
            dir.join(format!("2026-08-09_00-00-{i:02}.state")),
            [i as u8; 8],
        )
        .unwrap();
    }
    let r = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald");
    let mut a = app_playing_in(d.path(), "Emerald");
    a.apply_at(Action::SaveState, 1_000);
    assert_eq!(r.older().len(), 6);
    a.undo(2_000);
    let l = r.list().unwrap();
    assert_eq!(l.len(), RING_MAX - 1, "undo did not remove the new save");
    assert_eq!(r.older().len(), 6, "an old state was lost or duplicated");
    assert!(
        l.iter().all(|e| !r
            .older()
            .iter()
            .any(|o| o.file_stem().and_then(|s| s.to_str()) == Some(&e.stamp))),
        "undo copied a moved-aside state back into the ring"
    );
}

#[test]
fn closing_the_lid_saves_to_resume_and_leaves_the_manual_ring_alone() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let r = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald");
    for i in 0..RING_MAX {
        r.push(&[i as u8; 64], b"png", &format!("2026-08-09_00-00-{i:02}"))
            .unwrap();
    }
    let before = r.list().unwrap();
    let mut a = app_playing_in(d.path(), "Emerald");
    for _ in 0..3 {
        a.apply(Action::LidClose);
        a.apply(Action::LidOpen);
    }
    a.settle_saves();
    assert_eq!(
        r.list().unwrap(),
        before,
        "a lid close touched the manual states"
    );
    assert!(
        r.read_resume().unwrap().is_some(),
        "closing the lid did not write the suspend state"
    );
}
