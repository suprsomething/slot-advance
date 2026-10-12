//! Settings > Screen > Show Game Titles: the setting, which cart gets a title, where it
//! sits, and when its marquee runs, pauses and starts again.

mod common;

use common::{app_booting_at, clocked, CLOCK_IS_SET};
use slot::app::{App, Phase, STAR_PX};
use slot_gfx::TexId;
use slot_input::{Action, Btn};
use slot_store::{read_favourites, read_slot_state, write_slot_state, SlotState};
use slot_ui::{
    display_title, Draw, QuickRow, QuickValue, ShownTitle, TitleTex, CART_W, MARQUEE_DELAY_S,
    OUT_W, TITLE_FACE_H, TITLE_VIEW_W,
};
use tempfile::TempDir;

const STEMS: [&str; 5] = [
    "Advance Wars (Europe) [!]",
    "Golden Sun (USA, Europe)",
    "Metroid Fusion (USA) (Rev 1)",
    "Pokemon - FireRed Version (USA)",
    "Pokemon Unbound (v2.1.1)",
];

fn library() -> TempDir {
    let d = common::tmp_root_with_carts(&STEMS);
    clocked(d.path());
    d
}

fn boot(d: &TempDir) -> App {
    let (a, _clock) = app_booting_at(d.path(), CLOCK_IS_SET);
    assert!(matches!(a.phase(), Phase::Shelf), "{:?}", a.phase());
    a
}

/// Lets time pass a frame at a time.
fn run(a: &mut App, secs: f32) {
    for _ in 0..(secs * 60.0).round() as usize {
        a.update(1.0 / 60.0);
    }
}

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

/// Settings > Screen > Show Game Titles, then a press of an arrow, then back out.
fn toggle_titles(a: &mut App) {
    a.apply(Action::QuickMenu);
    press(a, Btn::A);
    while a.quick_menu() != Some(QuickRow::Titles) {
        press(a, Btn::Down);
    }
    press(a, Btn::Right);
    press(a, Btn::B);
    press(a, Btn::B);
    assert_eq!(a.quick_menu(), None);
}

const SHORT: usize = 5_001;
const FULL: usize = 5_002;

/// A pretend title, as the frontend builds: a long one has a whole line to scroll.
fn title(long: bool, n: usize) -> ShownTitle {
    ShownTitle {
        short: TitleTex {
            tex: TexId::from_raw(SHORT + 10 * n),
            w: if long { 270.0 } else { 120.0 },
            h: TITLE_FACE_H as f32,
        },
        full: long.then_some(TitleTex {
            tex: TexId::from_raw(FULL + 10 * n),
            w: 640.0,
            h: TITLE_FACE_H as f32,
        }),
    }
}

/// Hands the selected cart its title, if it wants one, as the frontend does each frame.
fn give(a: &mut App, long: bool, n: &mut usize) -> Vec<TexId> {
    let mut back = a.shed_titles();
    if let Some((shelf, index)) = a.title_wants() {
        *n += 1;
        back.extend(a.set_cart_title(shelf, index, title(long, *n)));
    }
    back
}

/// One frame: sync titles, then advance.
fn frame(a: &mut App, long: bool, n: &mut usize) {
    give(a, long, n);
    a.update(1.0 / 60.0);
}

fn is_title(t: TexId) -> bool {
    (SHORT..6_000).contains(&raw(t))
}

fn raw(t: TexId) -> usize {
    let s = format!("{t:?}");
    s.trim_start_matches("TexId(")
        .trim_end_matches(')')
        .parse()
        .unwrap()
}

/// Title draws: (x, y, w, h, texture, scrolling).
fn title_draws(a: &App) -> Vec<(f32, f32, f32, f32, TexId, bool)> {
    let mut out = Vec::new();
    a.draw(&mut out);
    out.into_iter()
        .filter_map(|d| match d {
            Draw::Tex {
                x, y, w, h, tex, ..
            } if is_title(tex) => Some((x, y, w, h, tex, false)),
            Draw::Part {
                x, y, w, h, tex, ..
            } if is_title(tex) => Some((x, y, w, h, tex, true)),
            _ => None,
        })
        .collect()
}

/// How many different titles are drawn (a scrolling one is drawn in several strips).
fn shown_titles(a: &App) -> usize {
    let faces: std::collections::BTreeSet<_> = title_draws(a)
        .iter()
        .map(|t| (raw(t.4) - SHORT) / 10)
        .collect();
    faces.len()
}

fn scrolling(a: &App) -> bool {
    title_draws(a).iter().any(|d| d.5)
}

#[test]
fn titles_are_off_by_default_and_nothing_is_asked_for() {
    let d = library();
    let a = boot(&d);
    assert!(!a.show_titles());
    assert_eq!(a.quick_value(QuickRow::Titles), Some(QuickValue::Off));
    assert!(a.title_wants().is_none());
    assert!(!read_slot_state(d.path()).show_titles);
}

/// Settings > Screen > Show Game Titles and back out, flipping it or not.
fn visit_titles(a: &mut App, flip: bool) {
    a.apply(Action::QuickMenu);
    press(a, Btn::A);
    while a.quick_menu() != Some(QuickRow::Titles) {
        press(a, Btn::Down);
    }
    if flip {
        press(a, Btn::Right);
    }
    press(a, Btn::B);
    press(a, Btn::B);
}

#[test]
fn off_draws_exactly_what_v3_drew() {
    let d = library();
    let mut plain = boot(&d);
    let mut a = boot(&d);
    let mut before = Vec::new();
    a.draw(&mut before);
    let mut first = Vec::new();
    plain.draw(&mut first);
    assert_eq!(before, first);
    // The same presses and the same time on both, but only one ever shows a title.
    visit_titles(&mut a, true);
    visit_titles(&mut plain, false);
    let mut n = 0;
    give(&mut a, true, &mut n);
    run(&mut a, MARQUEE_DELAY_S + 1.0);
    run(&mut plain, MARQUEE_DELAY_S + 1.0);
    assert!(scrolling(&a));
    visit_titles(&mut a, true);
    visit_titles(&mut plain, false);
    assert!(!a.show_titles());
    let shed = a.shed_titles();
    assert_eq!(
        shed.len(),
        2,
        "both title faces come back when titles go off"
    );
    let (mut after, mut expected) = (Vec::new(), Vec::new());
    a.draw(&mut after);
    plain.draw(&mut expected);
    assert_eq!(after, expected, "the carousel changed with titles off");
}

#[test]
fn on_takes_effect_at_once_saves_and_survives_a_restart() {
    let d = library();
    let mut a = boot(&d);
    toggle_titles(&mut a);
    assert!(a.show_titles());
    assert!(
        a.title_wants().is_some(),
        "nothing asked for straight after switching on"
    );
    assert!(read_slot_state(d.path()).show_titles, "not saved");
    drop(a);
    let again = boot(&d);
    assert!(again.show_titles(), "forgotten after a restart");
    assert!(again.title_wants().is_some());
}

#[test]
fn only_the_selected_cart_has_a_title_centred_above_it() {
    let d = library();
    let mut a = boot(&d);
    toggle_titles(&mut a);
    let mut n = 0;
    give(&mut a, false, &mut n);
    let titles = title_draws(&a);
    assert_eq!(titles.len(), 1, "a title on more than the selected cart");
    assert_eq!(shown_titles(&a), 1);
    let (x, _, w, _, _, _) = titles[0];
    assert!(
        ((x + w / 2.0) - OUT_W as f32 / 2.0).abs() < 0.5,
        "not centred on the cart"
    );
    assert!(w <= TITLE_VIEW_W as f32 && (TITLE_VIEW_W as f32) < CART_W as f32);
    assert!(a.title_wants().is_none(), "asked again for a title it has");
}

#[test]
fn a_long_title_holds_still_then_scrolls_inside_its_window() {
    let d = library();
    let mut a = boot(&d);
    toggle_titles(&mut a);
    let mut n = 0;
    give(&mut a, true, &mut n);
    run(&mut a, MARQUEE_DELAY_S - 0.2);
    assert!(!scrolling(&a), "scrolled before the delay");
    run(&mut a, 0.6);
    assert!(scrolling(&a), "did not scroll after the delay");
    let (lo, hi) = (
        OUT_W as f32 / 2.0 - TITLE_VIEW_W as f32 / 2.0,
        OUT_W as f32 / 2.0 + TITLE_VIEW_W as f32 / 2.0,
    );
    for _ in 0..1200 {
        a.update(1.0 / 60.0);
        for (x, _, w, _, _, _) in title_draws(&a) {
            assert!(
                x >= lo - 0.01 && x + w <= hi + 0.01,
                "drawn outside the window"
            );
        }
    }
}

#[test]
fn moving_to_another_cart_resets_the_marquee_at_once() {
    let d = library();
    let mut a = boot(&d);
    toggle_titles(&mut a);
    let mut n = 0;
    give(&mut a, true, &mut n);
    run(&mut a, MARQUEE_DELAY_S + 1.0);
    assert!(scrolling(&a));
    press(&mut a, Btn::Right);
    assert!(
        title_draws(&a).is_empty(),
        "the old title is still drawn after moving on"
    );
    assert!(a.title_wants().is_some(), "the new cart was not asked for");
    let back = give(&mut a, true, &mut n);
    assert!(
        back.len() >= 2,
        "the old title's faces were not handed back: {back:?}"
    );
    a.update(1.0 / 60.0);
    assert!(
        a.title_marquee() < 0.1,
        "the marquee carried on: {}",
        a.title_marquee()
    );
    assert!(!scrolling(&a), "the new title scrolled straight away");
    run(&mut a, MARQUEE_DELAY_S + 0.5);
    assert!(scrolling(&a), "the new title never scrolled");
}

#[test]
fn rapid_navigation_never_shows_a_stale_or_doubled_title() {
    let d = library();
    let mut a = boot(&d);
    toggle_titles(&mut a);
    let mut n = 0;
    for step in 0..40 {
        // A title made for the cart selected a moment ago arrives late.
        let late = a.title_wants();
        a.apply(if step % 7 == 3 {
            Action::ShelfLeft
        } else {
            Action::ShelfRight
        });
        if let Some((shelf, index)) = late {
            n += 1;
            let back = a.set_cart_title(shelf, index, title(true, n));
            assert_eq!(back.len(), 2, "a late title for the old cart was kept");
        }
        frame(&mut a, step % 2 == 0, &mut n);
        assert!(
            shown_titles(&a) <= 1,
            "two titles on screen at once at step {step}"
        );
        assert!(!scrolling(&a), "scrolling during rapid navigation");
    }
}

#[test]
fn the_marquee_stops_while_a_menu_is_open_and_starts_afresh_after() {
    let d = library();
    let mut a = boot(&d);
    toggle_titles(&mut a);
    let mut n = 0;
    give(&mut a, true, &mut n);
    run(&mut a, MARQUEE_DELAY_S + 1.0);
    assert!(scrolling(&a));
    a.apply(Action::QuickMenu);
    run(&mut a, 6.0);
    assert!(a.title_marquee() < 0.05, "kept running behind the menu");
    assert!(!scrolling(&a), "scrolling behind the menu");
    press(&mut a, Btn::B);
    assert_eq!(a.quick_menu(), None);
    a.update(1.0 / 60.0);
    assert!(
        !scrolling(&a),
        "picked up mid-scroll instead of from the start"
    );
    run(&mut a, MARQUEE_DELAY_S + 0.5);
    assert!(scrolling(&a), "never started again after the menu closed");
}

#[test]
fn the_marquee_stops_for_sleep_and_starts_afresh_on_waking() {
    let d = library();
    let mut a = boot(&d);
    toggle_titles(&mut a);
    let mut n = 0;
    give(&mut a, true, &mut n);
    run(&mut a, MARQUEE_DELAY_S + 1.0);
    assert!(scrolling(&a));
    a.apply(Action::LidClose);
    run(&mut a, 2.0);
    assert!(a.title_marquee() < 0.05, "kept running with the lid shut");
    a.apply(Action::LidOpen);
    run(&mut a, 1.0);
    give(&mut a, true, &mut n);
    a.update(1.0 / 60.0);
    assert!(!scrolling(&a), "carried on scrolling after waking");
    run(&mut a, MARQUEE_DELAY_S + 0.5);
    assert!(scrolling(&a), "never scrolled again after waking");
}

#[test]
fn every_category_gets_the_same_title_treatment() {
    let d = library();
    let mut a = boot(&d);
    press(&mut a, Btn::Y); // a favourite, so the Favourites category has a cart
    a.apply(Action::Insert);
    run(&mut a, 0.1);
    let mut a = boot(&d);
    toggle_titles(&mut a);
    let mut n = 0;
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..4 {
        if let Some((shelf, _)) = a.title_wants() {
            seen.insert(shelf);
        }
        give(&mut a, true, &mut n);
        assert_eq!(shown_titles(&a), 1, "not one title on this category");
        run(&mut a, MARQUEE_DELAY_S + 0.5);
        assert!(scrolling(&a), "no marquee on this category");
        press(&mut a, Btn::R1);
        run(&mut a, 0.6);
    }
    assert!(
        seen.len() >= 2,
        "titles were only ever asked for on one category: {seen:?}"
    );
}

#[test]
fn the_star_sits_above_the_title_and_never_jumps() {
    let d = library();
    let mut a = boot(&d);
    let star = TexId::from_raw(4_242);
    a.set_star_face(star, STAR_PX as u32, STAR_PX as u32);
    press(&mut a, Btn::Y);
    let star_y = |a: &App| {
        let mut out = Vec::new();
        a.draw(&mut out);
        out.into_iter()
            .filter_map(|d| match d {
                Draw::Tex { y, tex, .. } if tex == star => Some(y),
                _ => None,
            })
            .fold(f32::MAX, f32::min)
    };
    let plain = star_y(&a);
    assert!(plain < f32::MAX, "the starred cart has no star");
    toggle_titles(&mut a);
    let waiting = star_y(&a);
    let mut n = 0;
    give(&mut a, false, &mut n);
    let titled = star_y(&a);
    assert!(titled < plain, "the star did not move up for the title");
    assert_eq!(waiting, titled, "the star jumped when the title arrived");
    let title_top = title_draws(&a).iter().map(|d| d.1).fold(f32::MAX, f32::min);
    assert!(
        titled + STAR_PX <= title_top + 0.5,
        "the star overlaps the title"
    );
}

#[test]
fn a_cart_with_no_artwork_still_gets_a_title() {
    let d = library();
    let mut a = boot(&d);
    toggle_titles(&mut a);
    assert!(
        a.title_wants().is_some(),
        "a cart without artwork was not offered a title"
    );
    let mut n = 0;
    give(&mut a, false, &mut n);
    assert_eq!(title_draws(&a).len(), 1);
}

#[test]
fn titles_never_change_a_cart_or_where_its_things_are_kept() {
    let d = library();
    let mut a = boot(&d);
    toggle_titles(&mut a);
    press(&mut a, Btn::Y);
    let stems: Vec<String> = a.carts().map(|c| c.stem.clone()).collect();
    for stem in &stems {
        assert!(STEMS.contains(&stem.as_str()), "a stem changed: {stem}");
    }
    let favourites = read_favourites(d.path());
    assert!(
        favourites
            .iter()
            .all(|f| STEMS.contains(&f.trim_start_matches("GBA/"))),
        "a favourite was stored under its display title: {favourites:?}"
    );
    let shown: Vec<String> = stems.iter().map(|s| display_title(s)).collect();
    assert!(shown.contains(&"Pokemon - FireRed Version".to_string()));
    assert!(shown.contains(&"Metroid Fusion".to_string()));
}

#[test]
fn an_older_settings_file_keeps_titles_off() {
    let d = library();
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            ..Default::default()
        },
    )
    .unwrap();
    let path = d.path().join("Config/slot.state");
    let text: String = std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with("show_titles"))
        .map(|l| format!("{l}\n"))
        .collect();
    std::fs::write(&path, text).unwrap();
    assert!(!boot(&d).show_titles());
}
