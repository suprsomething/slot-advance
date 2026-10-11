//! The game title shown above the selected cart on the carousel (Settings > Screen > Show
//! Game Titles), and its slow marquee for titles too long for their window.
//!
//! The title comes from the ROM's file name, which is also the cart's identity (its stem)
//! for artwork, favourites, recents, save states and profiles. Only the text drawn on
//! screen is cleaned: the stem itself is never changed.
//!
//! `display_title` keeps the title as it was written, punctuation and all, and only drops
//! what dump naming conventions append at the end: region, language, revision, version
//! and status groups in round brackets, and every square-bracket flag. A trailing round
//! group that does not read as metadata, such as `(Arcade)` or `(2nd Edition)`, is part of
//! the title and stays. (The generated cart labels have their own, stricter cleaning,
//! `clean_label`, which is unchanged.)

use crate::draw::{Draw, TexId};
use crate::plate::UndoFace;
use crate::text;

/// Height of a title face; the text sits centred in it.
pub const TITLE_FACE_H: u32 = 24;

/// Space between a title and the top of its cart, at full cart size.
pub const TITLE_GAP: f32 = 10.0;

/// The window a title is shown in, centred over the selected cart. Nothing is drawn
/// outside it.
pub const TITLE_VIEW_W: u32 = 280;

/// A long title holds still, cut short with an ellipsis, for this long after its cart is
/// selected,
pub const MARQUEE_DELAY_S: f32 = 2.0;
/// then scrolls its whole length right to left at this many pixels a second,
pub const MARQUEE_SPEED: f32 = 28.0;
/// then rests on its last words for this long before starting again.
pub const MARQUEE_END_S: f32 = 1.5;

/// While the title scrolls, it fades out over this much of the window at a cut edge,
const FADE_W: f32 = 14.0;
/// in this many steps.
const FADE_STEPS: usize = 4;

/// No title face is wider than this, whatever the file is called.
const MAX_FACE_W: u32 = 4096;

const TITLE_PX: f32 = 15.0;

/// A touch dimmer than the hint text, so titles sit behind the carts they name.
const TITLE_INK: [u8; 3] = [0xd6, 0xd2, 0xca];

const ELLIPSIS: &str = "\u{2026}";

/// The title as it should be read on screen, from a ROM's file name. A known ROM
/// extension, if the name still has one, is dropped too.
pub fn display_title(name: &str) -> String {
    let mut t = strip_extension(name.trim()).to_string();
    loop {
        let trimmed = trim_tail(&t);
        let Some((start, group, square)) = last_group(trimmed) else {
            break;
        };
        if !(square || is_metadata(group)) {
            break;
        }
        t = trim_tail(&trimmed[..start]).to_string();
    }
    let out = trim_tail(&t);
    if out.is_empty() {
        name.trim().to_string()
    } else {
        out.to_string()
    }
}

const ROM_EXTENSIONS: [&str; 6] = [".gba", ".gbc", ".gb", ".agb", ".zip", ".7z"];

fn strip_extension(name: &str) -> &str {
    let lower = name.to_ascii_lowercase();
    for ext in ROM_EXTENSIONS {
        if lower.ends_with(ext) && name.len() > ext.len() {
            return &name[..name.len() - ext.len()];
        }
    }
    name
}

/// Trailing whitespace, and a separator left dangling once a group is gone
/// ("Game - (USA)" reads "Game", not "Game -").
fn trim_tail(s: &str) -> &str {
    let mut s = s.trim_end();
    while let Some(rest) = s.strip_suffix(" -").or_else(|| s.strip_suffix(" _")) {
        s = rest.trim_end();
    }
    s
}

/// The last bracketed group, when the text ends with one: where it opens, what is inside,
/// and whether it is square. Nested brackets are matched properly.
fn last_group(s: &str) -> Option<(usize, &str, bool)> {
    let close = s.chars().next_back()?;
    let (open, square) = match close {
        ')' => ('(', false),
        ']' => ('[', true),
        _ => return None,
    };
    let mut depth = 0i32;
    for (i, ch) in s.char_indices().rev() {
        if ch == close {
            depth += 1;
        } else if ch == open {
            depth -= 1;
            if depth == 0 {
                return Some((i, &s[i + 1..s.len() - 1], square));
            }
        }
    }
    None
}

/// Whether a round-bracket group is dump metadata rather than part of the title: every
/// comma-separated part must be a region, a language code, a revision or version, a
/// release status or a date.
fn is_metadata(group: &str) -> bool {
    let parts: Vec<&str> = group.split([',', '+']).map(str::trim).collect();
    !parts.is_empty() && parts.iter().all(|p| !p.is_empty() && is_tag(p))
}

const REGIONS: [&str; 44] = [
    "usa",
    "europe",
    "japan",
    "world",
    "australia",
    "brazil",
    "canada",
    "china",
    "france",
    "germany",
    "hong kong",
    "italy",
    "korea",
    "netherlands",
    "spain",
    "sweden",
    "asia",
    "scandinavia",
    "russia",
    "taiwan",
    "uk",
    "unknown",
    "denmark",
    "finland",
    "norway",
    "portugal",
    "greece",
    "poland",
    "mexico",
    "latin america",
    "new zealand",
    "switzerland",
    "austria",
    "belgium",
    "india",
    "ireland",
    "israel",
    "u",
    "e",
    "j",
    "w",
    "eu",
    "us",
    "jp",
];

const STATUS: [&str; 28] = [
    "beta",
    "proto",
    "prototype",
    "demo",
    "sample",
    "unl",
    "unlicensed",
    "hack",
    "pirate",
    "kiosk",
    "alt",
    "debug",
    "promo",
    "virtual console",
    "aftermarket",
    "homebrew",
    "translated",
    "translation",
    "patched",
    "fixed",
    "trainer",
    "sgb enhanced",
    "gb compatible",
    "rumble version",
    "budget",
    "rerelease",
    "e-reader",
    "multiboot",
];

fn is_tag(p: &str) -> bool {
    let lower = p.to_lowercase();
    if REGIONS.contains(&lower.as_str()) || STATUS.contains(&lower.as_str()) {
        return true;
    }
    // A status with a number or letter after it: "Beta 2", "Alt 1", "Proto 3", "Demo A".
    if let Some((word, rest)) = lower.split_once(' ') {
        if STATUS.contains(&word) && is_short_id(rest) {
            return true;
        }
        // "Rev 1", "Rev A", "Rev 1.1", "Ver 1.0", "Version 1.1", "Disc 2", "Track 3".
        if ["rev", "ver", "ver.", "version", "disc", "disk", "track"].contains(&word)
            && is_short_id(rest)
        {
            return true;
        }
    }
    // "v1", "v1.1", "v2.1.1", "V1.0a", "Rev1".
    for prefix in ["v", "rev"] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            if rest.starts_with(|c: char| c.is_ascii_digit()) && is_version(rest) {
                return true;
            }
        }
    }
    // Language codes as No-Intro writes them: "En", "Fr", "Zh-Hant", "Pt-BR".
    if is_language(p) {
        return true;
    }
    // Translation tags: "T-En", "T+Eng", "T-En by Someone".
    if lower.starts_with("t-") || lower.starts_with("t+") {
        return true;
    }
    // Dates: "2004-10-01", "20041001", "2004-10".
    is_date(p)
}

fn is_short_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 6 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '.')
}

fn is_version(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c.is_ascii_lowercase())
        && s.chars().filter(|c| c.is_ascii_alphabetic()).count() <= 2
}

fn is_language(p: &str) -> bool {
    let mut parts = p.split('-');
    let Some(first) = parts.next() else {
        return false;
    };
    let code = |s: &str, upper_first: bool| {
        let c: Vec<char> = s.chars().collect();
        c.len() == 2
            && c[0].is_ascii_uppercase()
            && (c[1].is_ascii_lowercase() || (!upper_first && c[1].is_ascii_uppercase()))
    };
    if !code(first, true) {
        return false;
    }
    parts.all(|rest| {
        code(rest, false) || (rest.len() == 4 && rest.chars().all(|c| c.is_ascii_alphabetic()))
    })
}

fn is_date(p: &str) -> bool {
    let digits = p.chars().filter(|c| c.is_ascii_digit()).count();
    p.chars().all(|c| c.is_ascii_digit() || c == '-')
        && (digits == 6 || digits == 8)
        && (p.starts_with("19") || p.starts_with("20"))
}

/// The faces for one title: `short` fits the window, cut with an ellipsis if need be;
/// `full` is the whole line, made only when `short` had to be cut.
pub struct TitleFaces {
    pub short: UndoFace,
    pub full: Option<UndoFace>,
}

/// Draws the faces for a cleaned title, once per selection; the frontend uploads them.
pub fn title_faces(title: &str) -> TitleFaces {
    let line = title.to_uppercase();
    let short = title_face(&line, TITLE_VIEW_W);
    let fits = text::label_font().is_none_or(|font| {
        text::line_width(font, &line, TITLE_PX, text::tracking_for(TITLE_PX)) <= TITLE_VIEW_W as f32
    });
    let full = (!fits).then(|| title_face(&line, MAX_FACE_W));
    TitleFaces { short, full }
}

/// The line in the carousel's lettering, at most `max_w` wide, cut short with an ellipsis
/// when it would not fit. The text is not shrunk.
fn title_face(line: &str, max_w: u32) -> UndoFace {
    let h = TITLE_FACE_H;
    let Some(font) = text::label_font() else {
        return UndoFace {
            rgba: vec![0; (h * 4) as usize],
            w: 1,
            h,
        };
    };
    let px = TITLE_PX;
    let tracking = text::tracking_for(px);
    let line = fitted(font, line, px, tracking, max_w as f32);
    let w = (text::line_width(font, &line, px, tracking).ceil() as u32 + 2).clamp(1, max_w.max(1));
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let layout = text::Layout {
        lines: vec![line],
        px,
        tracking,
    };
    text::draw_centred(&mut rgba, w, h, &layout, TITLE_INK);
    UndoFace { rgba, w, h }
}

/// The line as drawn: whole if it fits, otherwise cut at a character and ended with an
/// ellipsis, never leaving a space before it.
fn fitted(font: &fontdue::Font, line: &str, px: f32, tracking: f32, max_w: f32) -> String {
    if text::line_width(font, line, px, tracking) <= max_w {
        return line.to_string();
    }
    let chars: Vec<char> = line.chars().collect();
    for keep in (1..chars.len()).rev() {
        let head: String = chars[..keep].iter().collect();
        let cut = format!(
            "{}{ELLIPSIS}",
            head.trim_end().trim_end_matches([' ', '-', ',', ':'])
        );
        if text::line_width(font, &cut, px, tracking) <= max_w {
            return cut;
        }
    }
    ELLIPSIS.to_string()
}

/// The line a short title face draws for `line` (already in capitals).
pub fn fitted_title(font: &fontdue::Font, line: &str) -> String {
    fitted(
        font,
        line,
        TITLE_PX,
        text::tracking_for(TITLE_PX),
        TITLE_VIEW_W as f32,
    )
}

/// An uploaded face and its size in pixels at full cart size.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct TitleTex {
    pub tex: TexId,
    pub w: f32,
    pub h: f32,
}

/// Where a title is in its cycle, `t` seconds after it appeared.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Marquee {
    /// The short face, still and centred.
    Still,
    /// The full line, its left edge `left` pixels from the window's left edge (at full
    /// cart size). Negative once it has scrolled.
    Scroll { left: f32 },
}

/// The marquee for a title whose short face is `short_w` wide and whole line `full_w`:
/// still for the delay, then scrolling at a steady speed until the end is in the window,
/// a rest there, then back to the start, for as long as the cart stays selected. The
/// scroll starts exactly where the short face sat, so nothing jumps.
pub fn marquee(t: f32, short_w: f32, full_w: f32) -> Marquee {
    let view = TITLE_VIEW_W as f32;
    if full_w <= view {
        return Marquee::Still;
    }
    let from = (view - short_w) / 2.0;
    let to = view - full_w;
    let run = (from - to) / MARQUEE_SPEED;
    let cycle = MARQUEE_DELAY_S + run + MARQUEE_END_S;
    let t = t.max(0.0) % cycle;
    if t < MARQUEE_DELAY_S {
        return Marquee::Still;
    }
    let left = (from - (t - MARQUEE_DELAY_S) * MARQUEE_SPEED).max(to);
    Marquee::Scroll { left }
}

/// The selected cart's title: its faces, ready to draw.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ShownTitle {
    pub short: TitleTex,
    pub full: Option<TitleTex>,
}

impl ShownTitle {
    /// Every texture this title holds.
    pub fn textures(&self) -> impl Iterator<Item = TexId> {
        std::iter::once(self.short.tex).chain(self.full.map(|f| f.tex))
    }

    /// Draws the title in its window above a cart whose top centre is `cx`, `top`, at
    /// `scale`, `t` seconds into its marquee. Returns the y of the window's top.
    pub fn draw(
        &self,
        cx: f32,
        top: f32,
        scale: f32,
        alpha: f32,
        t: f32,
        out: &mut Vec<Draw>,
    ) -> f32 {
        let h = TITLE_FACE_H as f32 * scale;
        let y = top - TITLE_GAP * scale - h;
        let view_x = cx - TITLE_VIEW_W as f32 * scale / 2.0;
        let full = self.full.filter(|f| f.w > TITLE_VIEW_W as f32);
        let state = full.map_or(Marquee::Still, |f| marquee(t, self.short.w, f.w));
        match (state, full) {
            (Marquee::Scroll { left }, Some(full)) => {
                for (a, b, fade) in visible_spans(left, full.w) {
                    out.push(Draw::Part {
                        x: view_x + a * scale,
                        y,
                        w: (b - a) * scale,
                        h,
                        tex: full.tex,
                        alpha: alpha * fade,
                        uv: [(a - left) / full.w, 0.0, (b - a) / full.w, 1.0],
                    });
                }
            }
            _ => {
                let w = self.short.w.min(TITLE_VIEW_W as f32) * scale;
                out.push(Draw::Tex {
                    x: cx - w / 2.0,
                    y,
                    w,
                    h,
                    tex: self.short.tex,
                    alpha,
                });
            }
        }
        y
    }
}

/// The parts of the window a line `full_w` wide, its left edge at `left`, shows: each as
/// window x from, x to and how faded, with soft steps at any edge the line runs past.
pub fn visible_spans(left: f32, full_w: f32) -> Vec<(f32, f32, f32)> {
    let view = TITLE_VIEW_W as f32;
    let (lo, hi) = (left.max(0.0), (left + full_w).min(view));
    if hi <= lo {
        return Vec::new();
    }
    let step = FADE_W / FADE_STEPS as f32;
    let mut cuts: Vec<(f32, f32, f32)> = Vec::new();
    let fade_left = left < 0.0;
    let fade_right = left + full_w > view;
    let mut mid = (lo, hi);
    if fade_left {
        for k in 0..FADE_STEPS {
            let fade = (k as f32 + 0.5) / FADE_STEPS as f32;
            cuts.push((k as f32 * step, (k + 1) as f32 * step, fade));
        }
        mid.0 = mid.0.max(FADE_W);
    }
    if fade_right {
        mid.1 = mid.1.min(view - FADE_W);
    }
    cuts.push((mid.0, mid.1, 1.0));
    if fade_right {
        for k in (0..FADE_STEPS).rev() {
            let fade = (k as f32 + 0.5) / FADE_STEPS as f32;
            cuts.push((view - (k + 1) as f32 * step, view - k as f32 * step, fade));
        }
    }
    cuts.into_iter()
        .map(|(a, b, f)| (a.max(lo), b.min(hi), f))
        .filter(|(a, b, _)| b - a > 0.01)
        .collect()
}
