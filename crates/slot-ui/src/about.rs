//! slot.advance's About screen: the boot logo, a few facts about this device (version,
//! games, space on the games card, battery), the project's address and the credits.
//!
//! It replaces upstream slot's "back of the console" sticker. The credits stay on the
//! device: slot.advance is based on slot by Brandon T. Kowalski, and emulation is by
//! mGBA and gpSP. The README carries the full credits, which the last line points to.

use slot_gfx::{Draw, TexId, OUT_H, OUT_W};

use crate::plate::UndoFace;
use crate::text;

/// The screen is drawn as one full-screen face.
pub const ABOUT_W: u32 = OUT_W;
pub const ABOUT_H: u32 = OUT_H;

const GROUND: [u8; 3] = [0x00, 0x00, 0x00];
const KEY: [u8; 3] = [0x96, 0x96, 0x96];
const VALUE: [u8; 3] = [0xf0, 0xf0, 0xf0];
const RULE: [u8; 3] = [0x2d, 0x2d, 0x2d];

const LOGO_W: u32 = 280;
const LOGO_TOP: u32 = 22;
const ROWS_X: u32 = 150;
const ROW_PX: f32 = 17.0;
const ROW_STEP: f32 = 34.0;
const URL_PX: f32 = 19.0;
const CREDIT_PX: f32 = 12.0;

/// The project's website, shown on the About screen.
pub const WEBSITE: &str = "SLOTADVANCE.COM";

/// The credit lines at the foot of the About screen.
pub const ABOUT_CREDITS: [&str; 2] = [
    "BASED ON SLOT BY BRANDON T. KOWALSKI",
    "EMULATION BY MGBA AND GPSP \u{b7} FULL CREDITS IN THE README",
];

/// Space on the games card, in bytes.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CardSpace {
    pub free: u64,
    pub total: u64,
}

/// An RGB image, row by row, used for the logo.
pub struct Logo<'a> {
    pub rgb: &'a [[u8; 3]],
    pub w: u32,
    pub h: u32,
}

pub struct AboutFields<'a> {
    pub version: &'a str,
    pub games: usize,
    pub card: Option<CardSpace>,
    pub battery: Option<u8>,
    pub logo: Option<Logo<'a>>,
}

/// A size in bytes as the card's maker would print it: decimal gigabytes with one decimal
/// place, or megabytes below one gigabyte.
pub fn card_size(bytes: u64) -> String {
    const GB: f64 = 1e9;
    const MB: f64 = 1e6;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.1} GB", b / GB)
    } else {
        format!("{:.0} MB", b / MB)
    }
}

/// The information rows, as (label, value) pairs, in screen order.
pub fn about_rows(f: &AboutFields) -> Vec<(String, String)> {
    let card = match f.card {
        Some(c) => format!("{} FREE OF {}", card_size(c.free), card_size(c.total)),
        None => "UNAVAILABLE".to_string(),
    };
    let battery = match f.battery {
        Some(p) => format!("{p}%"),
        None => "UNAVAILABLE".to_string(),
    };
    vec![
        ("VERSION".into(), f.version.to_uppercase()),
        ("GAMES".into(), f.games.to_string()),
        ("GAMES CARD".into(), card),
        ("BATTERY".into(), battery),
    ]
}

/// Every line of text the About screen shows, top to bottom.
pub fn about_lines(f: &AboutFields) -> Vec<String> {
    let mut out: Vec<String> = about_rows(f)
        .into_iter()
        .map(|(k, v)| format!("{k} {v}"))
        .collect();
    out.push(WEBSITE.to_string());
    out.extend(ABOUT_CREDITS.iter().map(|s| s.to_string()));
    out
}

struct Canvas {
    px: Vec<u8>,
    w: u32,
    h: u32,
}

impl Canvas {
    fn filled(w: u32, h: u32, c: [u8; 3]) -> Canvas {
        let mut px = vec![255u8; (w * h * 4) as usize];
        for p in px.chunks_exact_mut(4) {
            p[..3].copy_from_slice(&c);
        }
        Canvas { px, w, h }
    }

    fn rect(&mut self, x: u32, y: u32, w: u32, h: u32, c: [u8; 3]) {
        for yy in y..(y + h).min(self.h) {
            for xx in x..(x + w).min(self.w) {
                let at = ((yy * self.w + xx) * 4) as usize;
                self.px[at..at + 3].copy_from_slice(&c);
            }
        }
    }

    fn measure(&self, s: &str, px: f32) -> f32 {
        let Some(font) = text::label_font() else {
            return 0.0;
        };
        let layout = text::fit(font, s, f32::MAX, 1, px, px);
        layout
            .lines
            .first()
            .map(|l| text::line_width(font, l, px, layout.tracking))
            .unwrap_or(0.0)
    }

    fn print(&mut self, x: f32, y: f32, s: &str, px: f32, c: [u8; 3]) {
        let Some(font) = text::label_font() else {
            return;
        };
        let layout = text::fit(font, s, f32::MAX, 1, px, px);
        let measured = layout
            .lines
            .first()
            .map(|l| text::line_width(font, l, px, layout.tracking))
            .unwrap_or(0.0);
        let (bw, bh) = ((measured.ceil() as u32).max(1), (px * 1.6).ceil() as u32);
        let cov = text::coverage(bw, bh, &layout);
        let (x, y) = (x.max(0.0) as u32, y.max(0.0) as u32);
        for row in 0..bh {
            for col in 0..bw {
                let a = cov[(row * bw + col) as usize] as u32;
                if a == 0 {
                    continue;
                }
                let (dx, dy) = (x + col, y + row);
                if dx >= self.w || dy >= self.h {
                    continue;
                }
                let at = ((dy * self.w + dx) * 4) as usize;
                for (k, ink) in c.iter().enumerate() {
                    let under = self.px[at + k] as u32;
                    self.px[at + k] = ((*ink as u32 * a + under * (255 - a)) / 255) as u8;
                }
            }
        }
    }

    fn centred(&mut self, y: f32, s: &str, px: f32, c: [u8; 3]) {
        let w = self.measure(s, px);
        self.print((self.w as f32 - w) / 2.0, y, s, px, c);
    }

    /// Draws `logo` scaled to `w` wide, averaging the source pixels each output pixel covers.
    /// Returns the height drawn.
    fn logo(&mut self, logo: &Logo, x: u32, y: u32, w: u32) -> u32 {
        if logo.w == 0 || logo.h == 0 || logo.rgb.len() < (logo.w * logo.h) as usize {
            return 0;
        }
        let h = ((logo.h as u64 * w as u64) / logo.w as u64).max(1) as u32;
        for oy in 0..h {
            let sy0 = (oy as u64 * logo.h as u64 / h as u64) as u32;
            let sy1 = (((oy + 1) as u64 * logo.h as u64).div_ceil(h as u64) as u32)
                .clamp(sy0 + 1, logo.h);
            for ox in 0..w {
                let sx0 = (ox as u64 * logo.w as u64 / w as u64) as u32;
                let sx1 = (((ox + 1) as u64 * logo.w as u64).div_ceil(w as u64) as u32)
                    .clamp(sx0 + 1, logo.w);
                let mut sum = [0u32; 3];
                let mut n = 0u32;
                for sy in sy0..sy1 {
                    for sx in sx0..sx1 {
                        let p = logo.rgb[(sy * logo.w + sx) as usize];
                        for (total, channel) in sum.iter_mut().zip(p) {
                            *total += channel as u32;
                        }
                        n += 1;
                    }
                }
                let (dx, dy) = (x + ox, y + oy);
                if dx < self.w && dy < self.h {
                    let at = ((dy * self.w + dx) * 4) as usize;
                    for (dst, total) in self.px[at..at + 3].iter_mut().zip(sum) {
                        *dst = (total / n) as u8;
                    }
                }
            }
        }
        h
    }
}

pub fn about_face(f: &AboutFields) -> UndoFace {
    let mut c = Canvas::filled(ABOUT_W, ABOUT_H, GROUND);
    let mut y = LOGO_TOP as f32;
    if let Some(logo) = &f.logo {
        let drawn = c.logo(logo, (ABOUT_W - LOGO_W) / 2, LOGO_TOP, LOGO_W);
        y += drawn as f32 + 10.0;
    }

    let right = (ABOUT_W - ROWS_X) as f32;
    for (key, value) in about_rows(f) {
        c.print(ROWS_X as f32, y, &key, ROW_PX, KEY);
        let vw = c.measure(&value, ROW_PX);
        c.print(right - vw, y, &value, ROW_PX, VALUE);
        y += ROW_STEP;
        c.rect(ROWS_X, (y - 9.0) as u32, ABOUT_W - 2 * ROWS_X, 1, RULE);
    }

    y += 10.0;
    c.centred(y, WEBSITE, URL_PX, VALUE);
    y += 36.0;
    for line in ABOUT_CREDITS {
        c.centred(y, line, CREDIT_PX, KEY);
        y += 22.0;
    }

    UndoFace {
        rgba: c.px,
        w: ABOUT_W,
        h: ABOUT_H,
    }
}

pub fn draw_about(face: Option<TexId>, out: &mut Vec<Draw>) {
    let Some(tex) = face else {
        return;
    };
    out.push(Draw::Tex {
        x: 0.0,
        y: 0.0,
        w: ABOUT_W as f32,
        h: ABOUT_H as f32,
        tex,
        alpha: 1.0,
    });
}
