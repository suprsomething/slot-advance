use slot_ui::{about_face, about_lines, about_rows, card_size, AboutFields, CardSpace, Logo};

fn fields() -> AboutFields<'static> {
    AboutFields {
        version: "0.4.0",
        games: 142,
        card: Some(CardSpace {
            free: 21_400_000_000,
            total: 59_500_000_000,
        }),
        battery: Some(87),
        logo: None,
    }
}

#[test]
fn the_rows_show_version_games_card_and_battery_in_order() {
    let rows = about_rows(&fields());
    let keys: Vec<&str> = rows.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(keys, ["VERSION", "GAMES", "GAMES CARD", "BATTERY"]);
    assert_eq!(rows[0].1, "0.4.0");
    assert_eq!(rows[1].1, "142");
    assert_eq!(rows[2].1, "21.4 GB FREE OF 59.5 GB");
    assert_eq!(rows[3].1, "87%");
}

#[test]
fn a_pre_release_version_is_shown_whole() {
    let mut f = fields();
    f.version = "0.5.0-beta.1";
    assert_eq!(about_rows(&f)[0].1, "0.5.0-BETA.1");
}

#[test]
fn missing_readings_say_so_rather_than_showing_zero() {
    let mut f = fields();
    f.card = None;
    f.battery = None;
    let rows = about_rows(&f);
    assert_eq!(rows[2].1, "UNAVAILABLE");
    assert_eq!(rows[3].1, "UNAVAILABLE");
}

#[test]
fn card_sizes_read_as_the_card_maker_prints_them() {
    assert_eq!(card_size(64_000_000_000), "64.0 GB");
    assert_eq!(card_size(1_000_000_000), "1.0 GB");
    assert_eq!(card_size(512_000_000), "512 MB");
    assert_eq!(card_size(0), "0 MB");
}

#[test]
fn the_about_text_names_the_website_and_slots_sources() {
    let all = about_lines(&fields()).join("\n").to_uppercase();
    for owed in [
        "SLOTADVANCE.COM",
        "BASED ON SLOT",
        "BRANDON T. KOWALSKI",
        "MGBA",
        "GPSP",
        "FULL CREDITS IN THE README",
    ] {
        assert!(all.contains(owed), "the About text does not mention {owed}");
    }
}

#[test]
fn the_readme_carries_the_credits_the_about_screen_points_to() {
    let readme = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md"))
        .unwrap()
        .to_uppercase();
    let credits = &readme[readme.find("# CREDITS").expect("a Credits section")..];
    for owed in [
        "MGBA",
        "GPSP",
        "BASEOS",
        "PVAIBHAV",
        "OPEN SANS",
        "NERD FONTS",
        "RYAN L MCINTYRE",
        "LCD3X",
        "GIGAHERZ",
        "CART SOUNDS",
        "CLAUDE",
    ] {
        assert!(
            credits.contains(owed),
            "the README credits do not mention {owed}"
        );
    }
}

#[test]
fn the_face_is_full_screen_and_opaque() {
    let face = about_face(&fields());
    assert_eq!((face.w, face.h), (slot_gfx::OUT_W, slot_gfx::OUT_H));
    assert_eq!(face.rgba.len(), (face.w * face.h * 4) as usize);
    assert!(face.rgba.chunks_exact(4).all(|p| p[3] == 255));
}

#[test]
fn the_logo_is_drawn_centred_and_scaled_down() {
    // A 4x2 white logo becomes 280x140 in the middle of the top of the screen.
    let rgb = [[255u8; 3]; 8];
    let mut f = fields();
    f.logo = Some(Logo {
        rgb: &rgb,
        w: 4,
        h: 2,
    });
    let face = about_face(&f);
    let at = |x: u32, y: u32| face.rgba[((y * face.w + x) * 4) as usize];
    assert_eq!(at(360, 90), 255, "the middle of the logo is drawn");
    assert_eq!(at(219, 90), 0, "nothing left of the logo");
    assert_eq!(at(500, 90), 0, "nothing right of the logo");
    assert_eq!(at(360, 21), 0, "nothing above the logo");
}

#[test]
fn a_logo_that_does_not_match_its_size_is_skipped() {
    let rgb = [[255u8; 3]; 3];
    let mut f = fields();
    f.logo = Some(Logo {
        rgb: &rgb,
        w: 4,
        h: 2,
    });
    let face = about_face(&f);
    assert_eq!(face.rgba[((90 * face.w + 360) * 4) as usize], 0);
}
