//! Regenerate README screenshots from the real frontend and supplied artwork.
//! Run with: cargo run -p slot --no-default-features --example readme_screenshots
use slot::app::{App, STAR_PX};
use slot_gfx::{Compositor, HeadlessSurface, OUT_H, OUT_W};
use slot_input::{Action, Btn};
use slot_store::{write_slot_state, SlotState, StateEntry};
use slot_ui::{
    cart_face, cart_shadow, hint_face, photo_face, star_face, title_face, word_face, Polaroids,
    Printed,
};
use std::{fs, path::PathBuf};

fn load_faces(app: &mut App, c: &mut Compositor) {
    let row = app
        .carts()
        .map(|cart| {
            let f = cart_face(cart);
            c.create_texture(f.w, f.h, &f.rgba)
        })
        .collect();
    app.set_faces(row);
    for want in app.face_wants() {
        let cart = app.face_cart(want.shelf, want.index).unwrap().clone();
        let f = cart_face(&cart);
        let tex = c.create_texture(f.w, f.h, &f.rgba);
        app.set_cart_face(want.shelf, want.index, tex);
    }
}
fn settle(app: &mut App) {
    for _ in 0..200 {
        app.update(0.01);
    }
}
fn shot(app: &mut App, c: &mut Compositor, path: PathBuf) {
    load_faces(app, c);
    let text = app.slot_text().unwrap();
    let face = word_face(&text);
    let tex = c.create_texture(face.w, face.h, &face.rgba);
    app.set_shelf_platform_face(tex, face.w);
    for _ in 0..30 {
        app.update(0.01);
    }
    let mut draw = Vec::new();
    app.draw(&mut draw);
    c.begin_frame();
    c.draw_list(&draw);
    let pixels = c.read_frame();
    // Reject a frame captured during a carousel transition.
    let colour_at_centre = (210..310)
        .flat_map(|y| (240..480).map(move |x| (x, y)))
        .filter(|(x, y)| {
            let at = (y * OUT_W as usize + x) * 4;
            let rgb = &pixels[at..at + 3];
            *rgb.iter().max().unwrap() > 40
                && rgb.iter().max().unwrap() - rgb.iter().min().unwrap() > 20
        })
        .count();
    assert!(
        colour_at_centre > 8_000,
        "Centre cartridge is missing or still moving: {}",
        path.display()
    );
    let mut encoder = png::Encoder::new(fs::File::create(&path).unwrap(), OUT_W, OUT_H);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&pixels)
        .unwrap();
    println!("Wrote {}", path.display());
}
fn main() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let pack = repo.join("artwork/gba-originals");
    let out = repo.join(".github/resources/screenshots/advance");
    let root = tempfile::tempdir().unwrap();
    for dir in ["Games/GBA", "Labels/GBA", "Config"] {
        fs::create_dir_all(root.path().join(dir)).unwrap();
    }
    let names = [
        "Kirby - Nightmare in Dream Land",
        "Pokemon - Unbound",
        "Pokemon - Emerald Version (USA, Europe)",
    ];
    for name in names {
        let mut rom = vec![0u8; 256];
        rom[0xa0..0xa4].copy_from_slice(b"DEMO");
        fs::write(root.path().join(format!("Games/GBA/{name}.gba")), rom).unwrap();
        fs::copy(
            if name == names[1] {
                out.join("sources/pokemon-unbound-label.png")
            } else {
                pack.join(format!("{name}.png"))
            },
            root.path().join(format!("Labels/GBA/{name}.png")),
        )
        .unwrap();
    }
    fs::write(
        root.path().join("Config/favourites.txt"),
        format!("GBA/{}\nGBA/{}\n", names[1], names[2]),
    )
    .unwrap();
    fs::write(
        root.path().join("Config/recents.txt"),
        format!("GBA/{}\nGBA/{}\nGBA/{}\n", names[1], names[0], names[2]),
    )
    .unwrap();
    write_slot_state(
        root.path(),
        &SlotState {
            clock_set: true,
            ..Default::default()
        },
    )
    .unwrap();
    let surface =
        HeadlessSurface::new().expect("Headless rendering is required for screenshot export");
    let mut c = Compositor::new(&surface).unwrap();
    let mut app = App::boot(root.path());
    load_faces(&mut app, &mut c);
    let shadow = cart_shadow();
    let tex = c.create_texture(shadow.w, shadow.h, &shadow.rgba);
    app.set_cart_shadow(tex);
    let star = star_face(STAR_PX);
    let tex = c.create_texture(star.w, star.h, &star.rgba);
    app.set_star_face(tex, star.w, star.h);
    let clock = word_face("12:30");
    let tex = c.create_texture(clock.w, clock.h, &clock.rgba);
    app.set_shelf_clock_face(tex, clock.w);
    settle(&mut app);
    shot(&mut app, &mut c, out.join("all-unbound.png"));
    app.apply(Action::FfStart);
    app.apply(Action::FfStop);
    settle(&mut app);
    assert!(app.on_recents());
    shot(&mut app, &mut c, out.join("recents-unbound.png"));
    app.apply(Action::FfStart);
    app.apply(Action::FfStop);
    settle(&mut app);
    assert!(app.on_favourites());
    app.apply(Action::GbaDown(Btn::Right));
    app.apply(Action::GbaUp(Btn::Right));
    settle(&mut app);
    shot(&mut app, &mut c, out.join("favourites-unbound.png"));
    switcher_shots(&mut c, &out);
}

// Reuse the switcher's actual drawing code for both its normal and delete states.
fn switcher_shots(c: &mut Compositor, out: &std::path::Path) {
    c.set_screen_effect(slot_gfx::ScreenEffect::None);
    let entries = (0..5)
        .map(|i| StateEntry {
            stamp: format!("2026-10-10_12-{:02}-00", 22 - i),
            state: PathBuf::new(),
            thumb: out.join("sources/pokemon-unbound-hall-of-fame.png"),
        })
        .collect();
    let mut p = Polaroids::new(entries);
    p.index = 1;
    let f = photo_face(&p.entries[0]);
    let tex = c.create_texture(f.w, f.h, &f.rgba);
    p.set_faces(vec![tex; 5]);
    let f = word_face("12:30");
    let clock = Printed {
        face: Some(c.create_texture(f.w, f.h, &f.rgba)),
        w: f.w,
    };
    for (delete, name) in [
        (false, "switcher-unbound.png"),
        (true, "switcher-delete-unbound.png"),
    ] {
        if delete {
            p.ask_delete();
        }
        let f = title_face(&p.title("2026-10-10_12-30-00"));
        p.set_title_face(Some(c.create_texture(f.w, f.h, &f.rgba)));
        let faces = p
            .hints()
            .iter()
            .map(|h| {
                let f = hint_face(h.key, &h.label);
                c.create_texture(f.w, f.h, &f.rgba)
            })
            .collect();
        p.set_hint_faces(faces);
        let mut draw = Vec::new();
        p.draw(None, Printed::default(), None, clock, &mut draw);
        c.begin_frame();
        c.draw_list(&draw);
        let pixels = c.read_frame();
        let path = out.join(name);
        let mut encoder = png::Encoder::new(fs::File::create(&path).unwrap(), OUT_W, OUT_H);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&pixels)
            .unwrap();
        println!("Wrote {}", path.display());
    }
}
