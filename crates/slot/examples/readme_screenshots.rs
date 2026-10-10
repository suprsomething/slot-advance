//! Regenerate the README shelf screenshots from the real frontend and label pack.
//! Run with: cargo run -p slot --no-default-features --example readme_screenshots
use slot::app::{App, STAR_PX};
use slot_gfx::{Compositor, HeadlessSurface, OUT_H, OUT_W};
use slot_input::{Action, Btn};
use slot_store::{write_slot_state, SlotState};
use slot_ui::{cart_face, cart_shadow, star_face, word_face};
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
    for _ in 0..40 {
        app.update(0.05);
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
        "Mario Kart - Super Circuit (USA)",
        "Pokemon - Emerald Version (USA, Europe)",
    ];
    for name in names {
        let mut rom = vec![0u8; 256];
        rom[0xa0..0xa4].copy_from_slice(b"DEMO");
        fs::write(root.path().join(format!("Games/GBA/{name}.gba")), rom).unwrap();
        fs::copy(
            pack.join(format!("{name}.png")),
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
    app.apply(Action::GbaDown(Btn::Right));
    app.apply(Action::GbaUp(Btn::Right));
    settle(&mut app);
    shot(&mut app, &mut c, out.join("all-with-favourites.png"));
    app.apply(Action::FfStart);
    app.apply(Action::FfStop);
    settle(&mut app);
    assert!(app.on_recents());
    shot(&mut app, &mut c, out.join("recents.png"));
    app.apply(Action::FfStart);
    app.apply(Action::FfStop);
    settle(&mut app);
    assert!(app.on_favourites());
    shot(&mut app, &mut c, out.join("favourites.png"));
}
