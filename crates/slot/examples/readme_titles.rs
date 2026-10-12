//! Regenerate the README's game-title screenshots (Settings > Screen > Show Game Titles)
//! from the real frontend and the label starter pack: the same carousel with titles off
//! and on. Nothing here changes how slot advance behaves.
//! Run with: cargo run -p slot --no-default-features --example readme_titles
use slot::app::{App, STAR_PX};
use slot::title_faces::TitleFaces;
use slot_gfx::{Compositor, HeadlessSurface, OUT_H, OUT_W};
use slot_store::{write_slot_state, SlotState};
use slot_ui::{cart_face, cart_shadow, star_face, word_face};
use std::{fs, path::Path, path::PathBuf};

const GAMES: [&str; 3] = [
    "Legend of Zelda, The - The Minish Cap (USA)",
    "Pokemon - Fire Red Version (U) (V1.1)",
    "Metroid - Zero Mission (USA)",
];

fn boot(root: &Path, c: &mut Compositor, titles: bool) -> App {
    write_slot_state(
        root,
        &SlotState {
            clock_set: true,
            show_titles: titles,
            ..Default::default()
        },
    )
    .unwrap();
    let mut app = App::boot(root);
    let faces = app
        .carts()
        .map(|cart| {
            let f = cart_face(cart);
            c.create_texture(f.w, f.h, &f.rgba)
        })
        .collect();
    app.set_faces(faces);
    for want in app.face_wants() {
        let cart = app.face_cart(want.shelf, want.index).unwrap().clone();
        let f = cart_face(&cart);
        let tex = c.create_texture(f.w, f.h, &f.rgba);
        app.set_cart_face(want.shelf, want.index, tex);
    }
    let shadow = cart_shadow();
    let tex = c.create_texture(shadow.w, shadow.h, &shadow.rgba);
    app.set_cart_shadow(tex);
    let star = star_face(STAR_PX);
    let tex = c.create_texture(star.w, star.h, &star.rgba);
    app.set_star_face(tex, star.w, star.h);
    let clock = word_face("12:30");
    let tex = c.create_texture(clock.w, clock.h, &clock.rgba);
    app.set_shelf_clock_face(tex, clock.w);
    if let Some(text) = app.slot_text() {
        let face = word_face(&text);
        let tex = c.create_texture(face.w, face.h, &face.rgba);
        app.set_shelf_platform_face(tex, face.w);
    }
    app
}

fn write_png(pixels: &[u8], path: &Path) {
    let mut encoder = png::Encoder::new(fs::File::create(path).unwrap(), OUT_W, OUT_H);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(pixels)
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
    for name in GAMES {
        let mut rom = vec![0u8; 256];
        rom[0xa0..0xa4].copy_from_slice(b"DEMO");
        fs::write(root.path().join(format!("Games/GBA/{name}.gba")), rom).unwrap();
        fs::copy(
            pack.join(format!("{name}.png")),
            root.path().join(format!("Labels/GBA/{name}.png")),
        )
        .unwrap();
    }
    // Open the carousel on Pokémon FireRed, in the middle of the shelf.
    fs::write(
        root.path().join("Config/recents.txt"),
        format!("GBA/{}\n", GAMES[1]),
    )
    .unwrap();
    let surface =
        HeadlessSurface::new().expect("Headless rendering is required for screenshot export");
    let mut c = Compositor::new(&surface).unwrap();
    for (titles, name) in [(false, "titles-off.png"), (true, "titles-on.png")] {
        let mut app = boot(root.path(), &mut c, titles);
        let mut faces = TitleFaces::default();
        // One second in: the carousel has settled and a long title still shows cut short,
        // before its marquee starts.
        for _ in 0..60 {
            faces.sync(&mut app, &mut c);
            app.update(1.0 / 60.0);
        }
        let mut draw = Vec::new();
        app.draw(&mut draw);
        c.begin_frame();
        c.draw_list(&draw);
        write_png(&c.read_frame(), &out.join(name));
    }
}
