//! Settings > Screen > Show Game Titles: the setting, what the carousel asks for, and where
//! a title lands relative to its cart and the favourite star.

mod common;

use common::{app_booting_at, clocked, CLOCK_IS_SET};
use slot::app::{App, Phase, STAR_PX};
use slot_gfx::TexId;
use slot_input::{Action, Btn};
use slot_store::{read_slot_state, write_slot_state, SlotState};
use slot_ui::{
    display_title, Draw, QuickRow, QuickValue, TitleTex, CART_W, OUT_W, TITLE_FACE_H, TITLE_GAP,
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

/// Lets the carousel's scroll come to rest, a frame at a time.
fn settle(a: &mut App) {
    for _ in 0..60 {
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

/// Hands every wanted cart a pretend title face, as the frontend does with real ones.
fn give_titles(a: &mut App, next: &mut usize) -> usize {
    let wants = a.title_wants();
    for (shelf, index) in &wants {
        *next += 1;
        let title = TitleTex {
            tex: TexId::from_raw(5_000 + *next),
            w: 200.0,
            h: TITLE_FACE_H as f32,
        };
        assert_eq!(a.set_cart_title(*shelf, *index, title), None);
    }
    wants.len()
}

fn tex_draws(a: &App) -> Vec<(f32, f32, f32, f32, TexId)> {
    let mut out = Vec::new();
    a.draw(&mut out);
    out.into_iter()
        .filter_map(|d| match d {
            Draw::Tex {
                x, y, w, h, tex, ..
            } => Some((x, y, w, h, tex)),
            _ => None,
        })
        .collect()
}

fn title_draws(a: &App) -> Vec<(f32, f32, f32, f32, TexId)> {
    tex_draws(a)
        .into_iter()
        .filter(|d| (5_001..6_000).contains(&raw(d.4)))
        .collect()
}

fn raw(t: TexId) -> usize {
    let s = format!("{t:?}");
    s.trim_start_matches("TexId(")
        .trim_end_matches(')')
        .parse()
        .unwrap()
}

#[test]
fn titles_are_off_by_default_and_nothing_is_asked_for() {
    let d = library();
    let a = boot(&d);
    assert!(!a.show_titles());
    assert_eq!(a.quick_value(QuickRow::Titles), Some(QuickValue::Off));
    assert!(a.title_wants().is_empty());
    assert!(!read_slot_state(d.path()).show_titles);
}

#[test]
fn off_draws_exactly_what_v3_drew() {
    let d = library();
    let mut a = boot(&d);
    let before = {
        let mut out = Vec::new();
        a.draw(&mut out);
        out
    };
    toggle_titles(&mut a);
    let mut n = 0;
    give_titles(&mut a, &mut n);
    toggle_titles(&mut a);
    let shed = a.shed_titles();
    assert_eq!(
        shed.len(),
        n,
        "every title face comes back when titles go off"
    );
    let mut after = Vec::new();
    a.draw(&mut after);
    assert_eq!(before, after, "the carousel changed with titles off");
}

#[test]
fn on_takes_effect_at_once_saves_and_survives_a_restart() {
    let d = library();
    let mut a = boot(&d);
    toggle_titles(&mut a);
    assert!(a.show_titles());
    assert!(
        !a.title_wants().is_empty(),
        "nothing asked for straight after switching on"
    );
    assert!(read_slot_state(d.path()).show_titles, "not saved");
    drop(a);
    let again = boot(&d);
    assert!(again.show_titles(), "forgotten after a restart");
    assert!(!again.title_wants().is_empty());
}

#[test]
fn a_title_sits_centred_just_above_its_cart_and_inside_its_width() {
    let d = library();
    let mut a = boot(&d);
    toggle_titles(&mut a);
    let mut n = 0;
    give_titles(&mut a, &mut n);
    let titles = title_draws(&a);
    assert!(!titles.is_empty(), "no titles drawn");
    let centre = titles
        .iter()
        .find(|(x, _, w, _, _)| ((x + w / 2.0) - OUT_W as f32 / 2.0).abs() < 1.0)
        .expect("a title centred over the selected cart");
    assert!(centre.2 <= CART_W as f32, "wider than its cart");
    let cart_top = centre.1 + centre.3 + TITLE_GAP;
    let face = tex_draws(&a).into_iter().find(|(x, y, w, _, t)| {
        !titles.iter().any(|tt| tt.4 == *t)
            && ((x + w / 2.0) - OUT_W as f32 / 2.0).abs() < 1.0
            && (*y - cart_top).abs() < 1.0
    });
    assert!(
        face.is_some() || titles.len() > 1,
        "the title is not just above the cart"
    );
    for (x, _, w, _, _) in &titles {
        let left = x;
        let right = x + w;
        let others = titles
            .iter()
            .filter(|t| (t.0 - x).abs() > 1.0)
            .any(|t| t.0 < right && t.0 + t.2 > *left);
        assert!(!others, "two titles overlap");
    }
}

#[test]
fn navigating_with_titles_on_asks_only_for_the_carts_coming_on_screen() {
    let d = library();
    let mut a = boot(&d);
    toggle_titles(&mut a);
    let mut n = 0;
    give_titles(&mut a, &mut n);
    assert!(
        a.title_wants().is_empty(),
        "everything on screen already has a title"
    );
    for _ in 0..3 {
        a.apply(Action::ShelfRight);
        settle(&mut a);
        give_titles(&mut a, &mut n);
        assert!(a.title_wants().is_empty());
        assert!(!title_draws(&a).is_empty());
    }
    toggle_titles(&mut a);
    assert!(a.title_wants().is_empty());
    assert_eq!(
        a.shed_titles().len(),
        n,
        "switching off frees every title face"
    );
    a.apply(Action::ShelfLeft);
    settle(&mut a);
    assert!(
        a.title_wants().is_empty(),
        "titles asked for while switched off"
    );
    toggle_titles(&mut a);
    assert!(
        !a.title_wants().is_empty(),
        "switching back on mid-carousel asked for nothing"
    );
}

#[test]
fn the_star_moves_up_above_the_title() {
    let d = library();
    let mut a = boot(&d);
    let star = TexId::from_raw(4_242);
    a.set_star_face(star, STAR_PX as u32, STAR_PX as u32);
    a.apply(Action::GbaDown(Btn::Y));
    a.apply(Action::GbaUp(Btn::Y));
    let star_y = |a: &App| {
        tex_draws(a)
            .into_iter()
            .filter(|d| d.4 == star)
            .map(|d| d.1)
            .fold(f32::MAX, f32::min)
    };
    let plain = star_y(&a);
    assert!(plain < f32::MAX, "the starred cart has no star");
    toggle_titles(&mut a);
    let waiting = star_y(&a);
    let mut n = 0;
    give_titles(&mut a, &mut n);
    let titled = star_y(&a);
    assert!(titled < plain, "the star did not move up for the title");
    assert_eq!(
        waiting, titled,
        "the star jumped when the title face arrived"
    );
    let title_top = title_draws(&a)
        .into_iter()
        .map(|d| d.1)
        .fold(f32::MAX, f32::min);
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
    let wants = a.title_wants();
    assert!(
        !wants.is_empty(),
        "carts without artwork were not offered titles"
    );
    let mut n = 0;
    give_titles(&mut a, &mut n);
    assert!(!title_draws(&a).is_empty());
}

#[test]
fn the_title_comes_from_the_cart_and_never_changes_it() {
    let d = library();
    let a = boot(&d);
    let stems: Vec<String> = a.carts().map(|c| c.stem.clone()).collect();
    for stem in &stems {
        assert!(STEMS.contains(&stem.as_str()), "a stem changed: {stem}");
    }
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
