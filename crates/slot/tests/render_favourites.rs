#![cfg(any(target_os = "macos", target_os = "linux"))]

mod common;

use std::sync::{Mutex, MutexGuard, PoisonError};

use slot::app::{App, STAR_PX};
use slot_gfx::{Compositor, HeadlessSurface, OUT_H, OUT_W};
use slot_input::{Action, Btn};
use slot_store::{write_slot_state, SlotState};
use slot_ui::{cart_face, cart_shadow, star_face};
use tempfile::TempDir;

static GL: Mutex<()> = Mutex::new(());

fn compositor() -> Option<(MutexGuard<'static, ()>, HeadlessSurface, Compositor)> {
    let guard = GL.lock().unwrap_or_else(PoisonError::into_inner);
    let surface = HeadlessSurface::new().ok()?;
    let compositor = Compositor::new(&surface).ok()?;
    Some((guard, surface, compositor))
}

fn boot(d: &TempDir, c: &mut Compositor) -> App {
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            ..Default::default()
        },
    )
    .unwrap();
    let mut app = App::boot(d.path());
    let faces = app
        .carts()
        .map(|cart| {
            let f = cart_face(cart);
            c.create_texture(f.w, f.h, &f.rgba)
        })
        .collect();
    app.set_faces(faces);
    let shadow = cart_shadow();
    let shadow = c.create_texture(shadow.w, shadow.h, &shadow.rgba);
    app.set_cart_shadow(shadow);
    let star = star_face(STAR_PX);
    let tex = c.create_texture(star.w, star.h, &star.rgba);
    app.set_star_face(tex, star.w, star.h);
    app
}

fn shot(app: &App, c: &mut Compositor, name: &str) -> Vec<u8> {
    let mut out = Vec::new();
    app.draw(&mut out);
    c.begin_frame();
    c.draw_list(&out);
    let px = c.read_frame();
    if let Ok(dir) = std::env::var("SCRATCH_PNG_DIR") {
        let path = format!("{dir}/favourites-{name}.png");
        let file = std::fs::File::create(&path).expect("create png");
        let mut e = png::Encoder::new(std::io::BufWriter::new(file), OUT_W, OUT_H);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        e.write_header()
            .expect("png header")
            .write_image_data(&px)
            .expect("png data");
        println!("wrote {path}");
    }
    px
}

/// Gold pixels above the carts, where only a star can be: red and green well over blue.
/// A dimmed star on a side cart is a darker gold, so this matches the hue, not the shade.
fn gold_above_carts(px: &[u8]) -> usize {
    let band = 0..175usize;
    px.chunks_exact(4)
        .enumerate()
        .filter(|(i, _)| band.contains(&(i / OUT_W as usize)))
        .filter(|(_, p)| {
            let (r, g, b) = (p[0] as i32, p[1] as i32, p[2] as i32);
            r > 90 && g > 60 && r - b > 50 && g - b > 30
        })
        .count()
}

/// Fills the faces the app asks for, the way the frontend's face loader does, so a shelf
/// that is rebuilt at runtime (Favourites) is drawn with its real labels.
fn load_wanted_faces(app: &mut App, c: &mut Compositor) {
    for want in app.face_wants() {
        let Some(cart) = app.face_cart(want.shelf, want.index).cloned() else {
            continue;
        };
        let f = cart_face(&cart);
        let tex = c.create_texture(f.w, f.h, &f.rgba);
        app.set_cart_face(want.shelf, want.index, tex);
    }
}

fn settle(app: &mut App) {
    for _ in 0..20 {
        app.update(0.05);
    }
}

#[test]
fn a_starred_cart_wears_a_star_on_all_and_favourites_shows_only_starred_carts() {
    let Some((_g, _s, mut c)) = compositor() else {
        eprintln!("no GL on this host, skipping");
        return;
    };
    let d = common::tmp_root_with_carts(&["Advance Wars", "Emerald", "Fusion", "Golden Sun"]);
    let mut app = boot(&d, &mut c);
    settle(&mut app);
    let plain = shot(&app, &mut c, "0-all-nothing-starred");
    assert_eq!(gold_above_carts(&plain), 0, "a star with nothing starred");

    app.apply(Action::GbaDown(Btn::Y));
    app.apply(Action::GbaUp(Btn::Y));
    settle(&mut app);
    let starred = shot(&app, &mut c, "1-all-centre-starred");
    assert!(
        gold_above_carts(&starred) > 50,
        "no star above the starred cart"
    );

    app.apply(Action::GbaDown(Btn::Right));
    app.apply(Action::GbaUp(Btn::Right));
    settle(&mut app);
    let moved = shot(&app, &mut c, "2-all-star-on-the-left-neighbour");
    assert!(
        gold_above_carts(&moved) > 20,
        "the star did not follow its cart off centre"
    );

    app.apply(Action::FfStart);
    app.apply(Action::FfStop);
    settle(&mut app);
    load_wanted_faces(&mut app, &mut c);
    assert!(app.on_favourites());
    let fav = shot(&app, &mut c, "3-favourites");
    assert_eq!(
        gold_above_carts(&fav),
        0,
        "Favourites stars its carts, which says nothing there"
    );
}
