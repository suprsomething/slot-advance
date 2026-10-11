//! Titles shown above the carts: what is cleaned from a ROM's name, what is kept, and how
//! a long one is fitted.

use slot_ui::{cart_title_face, display_title, CART_W, TITLE_FACE_H, TITLE_INSET};

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

fn max_w() -> u32 {
    CART_W - 2 * TITLE_INSET
}

#[test]
fn a_short_title_is_drawn_whole_and_narrower_than_its_cart() {
    let f = cart_title_face("Golden Sun", max_w());
    assert_eq!(f.h, TITLE_FACE_H);
    assert!(f.w < max_w() / 2, "{} px for two short words", f.w);
    assert!(f.rgba.chunks(4).any(|p| p[3] > 0), "nothing was drawn");
}

#[test]
fn a_long_title_is_cut_to_its_cart_with_an_ellipsis() {
    let long =
        "The Legend of Zelda - A Link to the Past & Four Swords - Special Collectors Edition";
    let f = cart_title_face(long, max_w());
    assert!(f.w <= max_w(), "{} px is wider than the cart allows", f.w);
    let font = slot_ui::text::label_font().expect("the label font");
    let line = slot_ui::fitted_title(font, &long.to_uppercase(), max_w());
    assert!(line.ends_with('\u{2026}'), "no ellipsis: {line}");
    assert!(
        !line.contains(" \u{2026}"),
        "a space before the ellipsis: {line}"
    );
    assert!(line.starts_with("THE LEGEND OF ZELDA"));
}

#[test]
fn accented_and_unusual_characters_draw() {
    for t in ["Pokémon - Émeraude", "Ōkami 天", "Ça Ira!? #1 @ 50%"] {
        let f = cart_title_face(t, max_w());
        assert!(f.w <= max_w());
        assert!(f.rgba.chunks(4).any(|p| p[3] > 0), "nothing drawn for {t}");
    }
}
