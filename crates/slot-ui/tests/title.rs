//! Titles shown above the carts: what is cleaned from a ROM's name, what is kept, and how
//! a long one is fitted.

use slot_gfx::TexId;
use slot_ui::{
    display_title, fitted_title, marquee, title_faces, visible_spans, Draw, Marquee, ShownTitle,
    TitleTex, MARQUEE_DELAY_S, MARQUEE_END_S, MARQUEE_SPEED, TITLE_FACE_H, TITLE_VIEW_W,
};

#[test]
fn the_examples_from_the_brief() {
    let cases = [
        (
            "Pokemon - FireRed Version (USA).gba",
            "Pokemon - FireRed Version",
        ),
        ("Golden Sun (USA, Europe).gba", "Golden Sun"),
        ("Metroid Fusion (USA) (Rev 1).gba", "Metroid Fusion"),
        ("Advance Wars (Europe) [!].gba", "Advance Wars"),
        (
            "Castlevania - Aria of Sorrow (USA) [T-En].gba",
            "Castlevania - Aria of Sorrow",
        ),
        ("Pokemon Unbound (v2.1.1).gba", "Pokemon Unbound"),
        (
            "The Legend of Zelda - The Minish Cap (USA).gba",
            "The Legend of Zelda - The Minish Cap",
        ),
    ];
    for (name, want) in cases {
        assert_eq!(display_title(name), want, "for {name}");
    }
}

#[test]
fn a_stem_without_its_extension_cleans_the_same() {
    assert_eq!(display_title("Golden Sun (USA, Europe)"), "Golden Sun");
    assert_eq!(display_title("Metroid Fusion"), "Metroid Fusion");
}

#[test]
fn one_or_many_trailing_groups_go() {
    let cases = [
        ("Game (USA)", "Game"),
        ("Game (Europe) (En,Fr,De,Es,It)", "Game"),
        ("Game (Japan) (Rev 2) (Beta) [b1] [!]", "Game"),
        ("Game [T+Eng1.0_Someone] [h1C]", "Game"),
        ("Game (U) [!]", "Game"),
        ("Game (USA) (Virtual Console)", "Game"),
        ("Game (2004-10-01) (Proto)", "Game"),
        ("Game (Demo 2)", "Game"),
        ("Game (Zh-Hant) (Unl)", "Game"),
        ("Game (Ver 1.1)", "Game"),
        ("Game (V1.0a)", "Game"),
        ("Game (SGB Enhanced) (GB Compatible)", "Game"),
    ];
    for (name, want) in cases {
        assert_eq!(display_title(name), want, "for {name}");
    }
}

#[test]
fn brackets_that_belong_to_the_title_stay() {
    let cases = [
        ("Ms. Pac-Man (Arcade)", "Ms. Pac-Man (Arcade)"),
        (
            "Who Wants to Be a Millionaire (2nd Edition) (USA)",
            "Who Wants to Be a Millionaire (2nd Edition)",
        ),
        (
            "Shrek (Hassle at the Castle) (Europe)",
            "Shrek (Hassle at the Castle)",
        ),
        ("Game (Arcade) (USA) [!]", "Game (Arcade)"),
        ("Pokemon (Fan Remake) (v1.0)", "Pokemon (Fan Remake)"),
    ];
    for (name, want) in cases {
        assert_eq!(display_title(name), want, "for {name}");
    }
}

#[test]
fn brackets_inside_the_title_are_never_touched() {
    assert_eq!(
        display_title("Yu-Gi-Oh! (The) Sacred Cards (USA)"),
        "Yu-Gi-Oh! (The) Sacred Cards"
    );
    assert_eq!(
        display_title("[Bracketed] Start (USA)"),
        "[Bracketed] Start"
    );
}

#[test]
fn spelling_punctuation_numbers_and_accents_are_kept() {
    let cases = [
        ("Pokémon - Émeraude (France)", "Pokémon - Émeraude"),
        (
            "Mario & Luigi - Superstar Saga (USA, Australia)",
            "Mario & Luigi - Superstar Saga",
        ),
        (
            "Final Fantasy I & II - Dawn of Souls (USA)",
            "Final Fantasy I & II - Dawn of Souls",
        ),
        ("Wario Land 4 (USA, Europe)", "Wario Land 4"),
        (
            "Yu-Gi-Oh! 7 Trials to Glory (USA)",
            "Yu-Gi-Oh! 7 Trials to Glory",
        ),
        ("Mother 3 (Japan) [T-En by Tomato]", "Mother 3"),
        (
            "Kirby's Dream Land 2 (USA, Europe) (SGB Enhanced)",
            "Kirby's Dream Land 2",
        ),
        (
            "Tony Hawk's Pro Skater 2 (USA, Europe)",
            "Tony Hawk's Pro Skater 2",
        ),
        ("Spider-Man 2 (USA)", "Spider-Man 2"),
    ];
    for (name, want) in cases {
        assert_eq!(display_title(name), want, "for {name}");
    }
}

#[test]
fn spaces_and_dangling_separators_are_tidied_at_the_end_only() {
    assert_eq!(display_title("  Spaced  Title   (USA)  "), "Spaced  Title");
    assert_eq!(display_title("Game - (USA)"), "Game");
    assert_eq!(display_title("Game_(Europe)"), "Game_");
}

#[test]
fn a_name_that_is_all_metadata_is_shown_as_it_is_rather_than_blank() {
    assert_eq!(display_title("(USA) (Rev 1)"), "(USA) (Rev 1)");
    assert_eq!(display_title("[!]"), "[!]");
    assert_eq!(display_title(""), "");
}

#[test]
fn unbalanced_brackets_do_not_panic_or_eat_the_title() {
    assert_eq!(display_title("Broken (USA"), "Broken (USA");
    assert_eq!(display_title("Broken USA)"), "Broken USA)");
    assert_eq!(display_title("Nested (a (b) c) (USA)"), "Nested (a (b) c)");
}

#[test]
fn a_name_with_no_metadata_is_shown_as_its_stem() {
    for name in [
        "Kirby - Nightmare in Dream Land",
        "Drill Dozer.gba",
        "Wario Land 4",
        "ZOE",
    ] {
        assert_eq!(display_title(name), name.trim_end_matches(".gba"));
    }
}

#[test]
fn capitalisation_and_unicode_are_kept() {
    assert_eq!(
        display_title("pokémon ROUGE feu (France)"),
        "pokémon ROUGE feu"
    );
    assert_eq!(
        display_title("ポケットモンスター ルビー (Japan)"),
        "ポケットモンスター ルビー"
    );
    assert_eq!(display_title("Ōkami Den (Japan) (Rev 1)"), "Ōkami Den");
}

const VIEW: f32 = TITLE_VIEW_W as f32;

#[test]
fn a_short_title_fits_its_window_and_has_nothing_to_scroll() {
    let f = title_faces("Golden Sun");
    assert_eq!(f.short.h, TITLE_FACE_H);
    assert!(
        f.short.w < TITLE_VIEW_W / 2,
        "{} px for two short words",
        f.short.w
    );
    assert!(
        f.short.rgba.chunks(4).any(|p| p[3] > 0),
        "nothing was drawn"
    );
    assert!(f.full.is_none(), "a short title was given a line to scroll");
}

#[test]
fn a_long_title_starts_cut_with_an_ellipsis_and_keeps_its_whole_line_to_scroll() {
    let long = "Pokemon Mystery Dungeon - Red Rescue Team";
    let f = title_faces(long);
    assert!(
        f.short.w <= TITLE_VIEW_W,
        "{} px is wider than the window",
        f.short.w
    );
    let full = f.full.expect("no whole line for a long title");
    assert!(
        full.w > TITLE_VIEW_W,
        "the whole line is no longer than the window"
    );
    assert_eq!(
        full.h, TITLE_FACE_H,
        "the scrolling line is a different size"
    );
    let font = slot_ui::text::label_font().expect("the label font");
    let line = fitted_title(font, &long.to_uppercase());
    assert!(line.ends_with('\u{2026}'), "no ellipsis: {line}");
    assert!(
        !line.contains(" \u{2026}"),
        "a space before the ellipsis: {line}"
    );
    assert!(line.starts_with("POKEMON MYSTERY"), "{line}");
}

#[test]
fn width_is_measured_not_counted() {
    // Same number of characters, very different widths.
    let narrow = title_faces(&"I".repeat(30));
    let wide = title_faces(&"W".repeat(30));
    assert!(
        narrow.full.is_none(),
        "thirty narrow letters measured as too wide"
    );
    assert!(
        wide.full.is_some(),
        "thirty wide letters measured as fitting"
    );
}

#[test]
fn accented_and_unusual_characters_measure_and_draw() {
    for t in [
        "Pokémon - Émeraude",
        "Ōkami 天",
        "Ça Ira!? #1 @ 50%",
        "Ünïcödé Çhäräctérs Everywhere In A Very Long Title Indeed",
    ] {
        let f = title_faces(t);
        assert!(f.short.w <= TITLE_VIEW_W, "{t}");
        assert!(
            f.short.rgba.chunks(4).any(|p| p[3] > 0),
            "nothing drawn for {t}"
        );
        if let Some(full) = &f.full {
            assert!(full.w > TITLE_VIEW_W, "{t}");
        }
    }
}

/// A title 600 px long whose cut face is 270 px.
const SHORT_W: f32 = 270.0;
const FULL_W: f32 = 600.0;

fn left_at(t: f32) -> Option<f32> {
    match marquee(t, SHORT_W, FULL_W) {
        Marquee::Still => None,
        Marquee::Scroll { left } => Some(left),
    }
}

#[test]
fn a_short_title_never_moves() {
    for i in 0..400 {
        assert_eq!(marquee(i as f32 * 0.05, 120.0, 120.0), Marquee::Still);
    }
}

#[test]
fn a_long_title_holds_still_for_the_delay_then_scrolls() {
    assert_eq!(left_at(0.0), None);
    assert_eq!(
        left_at(MARQUEE_DELAY_S - 0.01),
        None,
        "scrolled before the delay"
    );
    let start = left_at(MARQUEE_DELAY_S + 0.001).expect("did not scroll after the delay");
    assert!(
        (start - (VIEW - SHORT_W) / 2.0).abs() < 0.1,
        "the scroll does not start where the cut title sat: {start}"
    );
}

#[test]
fn the_speed_is_steady_and_does_not_depend_on_the_frame_rate() {
    let a = MARQUEE_DELAY_S + 1.0;
    let b = MARQUEE_DELAY_S + 3.0;
    let moved = left_at(a).unwrap() - left_at(b).unwrap();
    assert!(
        (moved - 2.0 * MARQUEE_SPEED).abs() < 0.01,
        "{moved} px in two seconds"
    );
    assert!((25.0..=30.0).contains(&MARQUEE_SPEED));
    // Stepping there at 30, 60 or 144 frames a second lands in the same place.
    for fps in [30.0f32, 60.0, 144.0] {
        let mut t = 0.0f32;
        while t + 1.0 / fps <= b {
            t += 1.0 / fps;
        }
        let gap = (left_at(t).unwrap() - left_at(b).unwrap()).abs();
        assert!(
            gap <= MARQUEE_SPEED / fps + 0.01,
            "{fps} fps drifted {gap} px"
        );
    }
    // Smooth: it moves by fractions of a pixel between frames, not whole characters.
    let step = left_at(a).unwrap() - left_at(a + 1.0 / 60.0).unwrap();
    assert!(step > 0.0 && step < 1.0, "{step} px a frame");
}

#[test]
fn it_stops_with_the_end_in_view_rests_then_starts_again() {
    let run = ((VIEW - SHORT_W) / 2.0 - (VIEW - FULL_W)) / MARQUEE_SPEED;
    let end = MARQUEE_DELAY_S + run;
    let at_end = left_at(end + 0.01).unwrap();
    assert!(
        (at_end - (VIEW - FULL_W)).abs() < 0.01,
        "stopped at {at_end}"
    );
    assert!(
        at_end + FULL_W <= VIEW + 0.01,
        "the last letters are not all in view"
    );
    assert_eq!(
        left_at(end + MARQUEE_END_S - 0.01),
        Some(at_end),
        "did not rest on the end"
    );
    let cycle = end + MARQUEE_END_S;
    assert_eq!(left_at(cycle + 0.01), None, "did not go back to the start");
    assert_eq!(left_at(cycle + MARQUEE_DELAY_S - 0.01), None);
    assert!(
        left_at(cycle + MARQUEE_DELAY_S + 0.5).is_some(),
        "did not repeat"
    );
    // Never runs backwards within a cycle: no bouncing.
    let mut last = f32::MAX;
    let mut t = MARQUEE_DELAY_S;
    while t < cycle {
        let l = left_at(t).unwrap();
        assert!(l <= last + 1e-4, "moved back at {t}");
        last = l;
        t += 1.0 / 60.0;
    }
}

#[test]
fn the_window_never_shows_anything_outside_itself() {
    let mut left = 5.0;
    while left > VIEW - FULL_W - 1.0 {
        let spans = visible_spans(left, FULL_W);
        assert!(!spans.is_empty());
        let mut x = spans[0].0;
        for (a, b, fade) in &spans {
            assert!(*a >= 0.0 && *b <= VIEW + 1e-3, "{a}..{b} at left {left}");
            assert!(
                *a >= left - 1e-3 && *b <= left + FULL_W + 1e-3,
                "past the text"
            );
            assert!(
                (a - x).abs() < 1e-3,
                "a gap or overlap at {a} (left {left})"
            );
            assert!(*fade > 0.0 && *fade <= 1.0);
            x = *b;
        }
        left -= 0.37;
    }
}

#[test]
fn only_an_edge_the_text_runs_past_is_faded() {
    let start = visible_spans((VIEW - SHORT_W) / 2.0, FULL_W);
    assert_eq!(
        start.first().unwrap().2,
        1.0,
        "faded on the left before scrolling"
    );
    assert!(
        start.last().unwrap().2 < 1.0,
        "not faded on the right where the text runs on"
    );
    let end = visible_spans(VIEW - FULL_W, FULL_W);
    assert!(
        end.first().unwrap().2 < 1.0,
        "not faded on the left once scrolled"
    );
    assert_eq!(
        end.last().unwrap().2,
        1.0,
        "faded on the right with the end in view"
    );
}

fn shown(long: bool) -> ShownTitle {
    ShownTitle {
        short: TitleTex {
            tex: TexId::from_raw(1),
            w: if long { SHORT_W } else { 100.0 },
            h: TITLE_FACE_H as f32,
        },
        full: long.then_some(TitleTex {
            tex: TexId::from_raw(2),
            w: FULL_W,
            h: TITLE_FACE_H as f32,
        }),
    }
}

fn spans(out: &[Draw]) -> Vec<(f32, f32)> {
    out.iter()
        .map(|d| match *d {
            Draw::Tex { x, w, .. } | Draw::Part { x, w, .. } => (x, x + w),
            _ => panic!("a title drew something other than its faces"),
        })
        .collect()
}

#[test]
fn drawn_titles_stay_centred_and_inside_the_window_at_every_moment() {
    let (cx, top) = (360.0, 200.0);
    for scale in [1.0, 0.8] {
        let (lo, hi) = (cx - VIEW * scale / 2.0, cx + VIEW * scale / 2.0);
        let mut out = Vec::new();
        shown(false).draw(cx, top, scale, 1.0, 9.0, &mut out);
        let s = spans(&out);
        assert_eq!(s.len(), 1);
        assert!(
            ((s[0].0 + s[0].1) / 2.0 - cx).abs() < 0.01,
            "a short title is not centred"
        );
        let mut t = 0.0;
        while t < 40.0 {
            let mut out = Vec::new();
            let y = shown(true).draw(cx, top, scale, 1.0, t, &mut out);
            assert!(y < top, "the title is not above the cart");
            for (a, b) in spans(&out) {
                assert!(
                    a >= lo - 0.01 && b <= hi + 0.01,
                    "{a}..{b} outside {lo}..{hi} at {t} s"
                );
            }
            t += 0.1;
        }
    }
}

#[test]
fn a_long_title_is_still_and_centred_until_the_delay_is_over() {
    let mut out = Vec::new();
    shown(true).draw(360.0, 200.0, 1.0, 1.0, MARQUEE_DELAY_S - 0.05, &mut out);
    assert!(matches!(out.as_slice(), [Draw::Tex { tex, .. }] if *tex == TexId::from_raw(1)));
    let mut out = Vec::new();
    shown(true).draw(360.0, 200.0, 1.0, 1.0, MARQUEE_DELAY_S + 0.5, &mut out);
    assert!(
        out.iter()
            .all(|d| matches!(d, Draw::Part { tex, .. } if *tex == TexId::from_raw(2))),
        "the whole line is not what scrolls"
    );
}
