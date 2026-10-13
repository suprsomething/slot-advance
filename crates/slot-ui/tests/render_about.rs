//! Renders the About screen to a PNG for a visual check. Runs only when SCRATCH_PNG names
//! the output file; SCRATCH_LOGO may name a 24-bit BMP to use as the logo.
use slot_ui::{about_face, AboutFields, CardSpace, Logo};

fn bmp(path: &str) -> Option<(Vec<[u8; 3]>, u32, u32)> {
    let b = std::fs::read(path).ok()?;
    let u32_at = |i: usize| u32::from_le_bytes(b[i..i + 4].try_into().unwrap());
    let (w, h, off) = (u32_at(18), u32_at(22) as i32, u32_at(10) as usize);
    let (top_down, h) = (h < 0, h.unsigned_abs());
    let stride = ((w * 3 + 3) & !3) as usize;
    let mut out = Vec::with_capacity((w * h) as usize);
    for y in 0..h as usize {
        let row = if top_down { y } else { h as usize - 1 - y };
        for px in b[off + row * stride..].chunks_exact(3).take(w as usize) {
            out.push([px[2], px[1], px[0]]);
        }
    }
    Some((out, w, h))
}

#[test]
fn render_about() {
    let Ok(out) = std::env::var("SCRATCH_PNG") else {
        return;
    };
    let logo = std::env::var("SCRATCH_LOGO").ok().and_then(|p| bmp(&p));
    let face = about_face(&AboutFields {
        version: "0.4.0",
        games: 142,
        card: Some(CardSpace {
            free: 21_400_000_000,
            total: 59_500_000_000,
        }),
        battery: Some(87),
        logo: logo.as_ref().map(|(rgb, w, h)| Logo { rgb, w: *w, h: *h }),
    });
    let file = std::fs::File::create(out).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), face.w, face.h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .unwrap()
        .write_image_data(&face.rgba)
        .unwrap();
}
