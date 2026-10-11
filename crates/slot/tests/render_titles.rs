#![cfg(any(target_os = "macos", target_os = "linux"))]

//! Game titles on the real compositor: drawn above the carts when on, and not a pixel of
//! difference when off. Skips itself where there is no GL.

mod common;

use std::sync::{Mutex, MutexGuard, PoisonError};

use slot::app::{App, STAR_PX};
use slot::title_faces::TitleFaces;
use slot_gfx::{Compositor, HeadlessSurface, OUT_H, OUT_W};
use slot_input::{Action, Btn};
use slot_store::{write_slot_state, SlotState};
use slot_ui::{cart_face, cart_shadow, star_face, QuickRow};
use tempfile::TempDir;

static GL: Mutex<()> = Mutex::new(());

fn compositor() -> Option<(MutexGuard<'static, ()>, HeadlessSurface, Compositor)> {
    let guard = GL.lock().unwrap_or_else(PoisonError::into_inner);
    let surface = HeadlessSurface::new().ok()?;
    let compositor = Compositor::new(&surface).ok()?;
    Some((guard, surface, compositor))
}

const STEMS: [&str; 5] = [
    "Advance Wars (Europe) [!]",
    "Castlevania - Aria of Sorrow (USA) [T-En]",
    "Golden Sun (USA, Europe)",
    "Pokemon - FireRed Version (USA)",
    "The Legend of Zelda - A Link to the Past & Four Swords (USA, Australia)",
];

fn boot(d: &TempDir, c: &mut Compositor, titles: bool) -> App {
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            show_titles: titles,
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
        let path = format!("{dir}/titles-{name}.png");
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

fn settle(app: &mut App) {
    for _ in 0..60 {
        app.update(1.0 / 60.0);
    }
}

/// Light pixels in the band just above the selected cart, where only a title can be.
fn ink_above_centre(px: &[u8], top: usize, bottom: usize) -> usize {
    let (l, r) = (OUT_W as usize / 2 - 180, OUT_W as usize / 2 + 180);
    (top..bottom)
        .flat_map(|y| (l..r).map(move |x| (y, x)))
        .filter(|(y, x)| {
            let p = &px[(y * OUT_W as usize + x) * 4..][..3];
            p.iter().all(|v| *v > 150)
        })
        .count()
}

#[test]
fn titles_draw_above_the_carts_and_off_changes_nothing() {
    let Some((_g, _s, mut c)) = compositor() else {
        eprintln!("no GL on this host, skipping");
        return;
    };
    let d = common::tmp_root_with_carts(&STEMS);

    let mut off = boot(&d, &mut c, false);
    let plain = shot(&off, &mut c, "0-off");
    let mut faces = TitleFaces::default();
    faces.sync(&mut off, &mut c);
    let still_plain = shot(&off, &mut c, "0-off-after-sync");
    assert_eq!(plain, still_plain, "titles off, yet the carousel changed");

    let mut on = boot(&d, &mut c, true);
    let mut faces = TitleFaces::default();
    faces.sync(&mut on, &mut c);
    let titled = shot(&on, &mut c, "1-on");
    assert_ne!(plain, titled, "titles on, yet nothing was drawn");
    assert!(
        ink_above_centre(&titled, 90, 132) > 100,
        "no title above the selected cart"
    );
    assert_eq!(
        ink_above_centre(&plain, 90, 132),
        0,
        "something above the cart with titles off"
    );

    on.apply(Action::GbaDown(Btn::Y));
    on.apply(Action::GbaUp(Btn::Y));
    shot(&on, &mut c, "2-on-starred");

    for i in 0..STEMS.len() {
        on.apply(Action::ShelfRight);
        settle(&mut on);
        faces.sync(&mut on, &mut c);
        let px = shot(&on, &mut c, &format!("3-on-step-{i}"));
        assert!(ink_above_centre(&px, 90, 132) > 100, "step {i}: no title");
    }

    on.apply(Action::QuickMenu);
    on.apply(Action::GbaDown(Btn::A));
    on.apply(Action::GbaUp(Btn::A));
    while on.quick_menu() != Some(QuickRow::Titles) {
        on.apply(Action::GbaDown(Btn::Down));
        on.apply(Action::GbaUp(Btn::Down));
    }
    shot(&on, &mut c, "4-menu");
}
