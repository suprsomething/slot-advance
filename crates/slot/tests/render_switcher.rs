#![cfg(any(target_os = "macos", target_os = "linux"))]

//! Pictures of the save-state switcher and its delete question, for checking by eye
//! (`SCRATCH_PNG_DIR`), plus the one property that can be checked by pixels: the question
//! changes what the bottom plate says.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use slot::thumb;
use slot_gfx::{Compositor, HeadlessSurface, ScreenEffect, OUT_H, OUT_W};
use slot_retro::{ButtonMask, MockCore, RetroCore};
use slot_store::{Core, Platform, StateRing};
use slot_ui::{hint_face, photo_face, title_face, Polaroids, Printed, CONFIRM_LEGEND, LEGEND};

static GL: Mutex<()> = Mutex::new(());

fn compositor() -> Option<(MutexGuard<'static, ()>, HeadlessSurface, Compositor)> {
    let guard = GL.lock().unwrap_or_else(PoisonError::into_inner);
    let surface = HeadlessSurface::new().ok()?;
    let compositor = Compositor::new(&surface).ok()?;
    Some((guard, surface, compositor))
}

fn mock_frame(frames: u32) -> Vec<u8> {
    let mut core = MockCore::new();
    core.load(Path::new("mock")).expect("mock load");
    for _ in 0..frames {
        core.run_frame(ButtonMask::default());
    }
    core.video_xrgb8888().to_vec()
}

fn shot(c: &mut Compositor, p: &Polaroids, name: &str) -> Vec<u8> {
    let mut out = Vec::new();
    p.draw(None, Printed::default(), None, Printed::default(), &mut out);
    c.set_screen_power(1.0);
    c.begin_frame();
    c.draw_list(&out);
    let px = c.read_frame();
    if let Ok(dir) = std::env::var("SCRATCH_PNG_DIR") {
        let path = format!("{dir}/switcher-{name}.png");
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

fn bottom_plate(px: &[u8]) -> &[u8] {
    let from = (OUT_H as usize - 60) * OUT_W as usize * 4;
    &px[from..]
}

#[test]
fn the_delete_question_replaces_the_timestamp_and_the_hints() {
    let Some((_g, _s, mut c)) = compositor() else {
        eprintln!("no GL on this host, skipping");
        return;
    };
    c.set_screen_effect(ScreenEffect::None);
    let d = tempfile::tempdir().expect("tempdir");
    let ring = StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Mock");
    for (i, stamp) in [
        "2026-10-10_11-02-00",
        "2026-10-10_11-40-00",
        "2026-10-10_12-05-00",
    ]
    .iter()
    .enumerate()
    {
        let frame = mock_frame(10 + 40 * i as u32);
        ring.push(b"state", &thumb::png(&frame).expect("encode"), stamp)
            .expect("push");
    }
    let entries = ring.list().expect("list");
    let faces = entries
        .iter()
        .map(|e| {
            let f = photo_face(e);
            c.create_texture_nearest(f.w, f.h, &f.rgba)
        })
        .collect();
    let upload = |c: &mut Compositor, legend: &[(&str, &str)]| -> Vec<_> {
        legend
            .iter()
            .map(|(k, l)| {
                let f = hint_face(k, l);
                c.create_texture(f.w, f.h, &f.rgba)
            })
            .collect()
    };
    let legend = upload(&mut c, &LEGEND);
    let confirm = upload(&mut c, &CONFIRM_LEGEND);

    let now = "2026-10-10_12-10-00";
    let mut p = Polaroids::new(entries);
    p.set_faces(faces);
    p.set_hint_faces(legend.clone());
    let title = title_face(&p.title(now));
    p.set_title_face(Some(c.create_texture(title.w, title.h, &title.rgba)));
    let browsing = shot(&mut c, &p, "0-newest");

    p.right();
    let title = title_face(&p.title(now));
    p.set_title_face(Some(c.create_texture(title.w, title.h, &title.rgba)));
    shot(&mut c, &p, "1-down-one");

    p.ask_delete();
    p.set_hint_faces(confirm);
    let title = title_face(&p.title(now));
    p.set_title_face(Some(c.create_texture(title.w, title.h, &title.rgba)));
    let asking = shot(&mut c, &p, "2-delete-question");

    assert_ne!(
        bottom_plate(&browsing),
        bottom_plate(&asking),
        "the bottom plate did not change for the question"
    );
}
