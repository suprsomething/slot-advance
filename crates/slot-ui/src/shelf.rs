use slot_gfx::{Draw, TexId, OUT_H, OUT_W};
use slot_store::Cart;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::cart::{cart_box, gb_shell_of, label_colour, label_text, CART_H, CART_W};
use crate::hud::Millis;
use crate::silhouette::GbShell;
use crate::slot_chrome::{draw_empty_slot, MOUTH_H};
use crate::title::{ShownTitle, TITLE_FACE_H, TITLE_GAP};

const SIDE_SCALE: f32 = 0.78;
const SIDE_ALPHA: f32 = 0.55;

static SLACK: AtomicU32 = AtomicU32::new(0);

pub fn set_shelf_slack(px: f32) {
    SLACK.store(px.max(0.0).to_bits(), Ordering::Relaxed);
}

fn raise() -> f32 {
    f32::from_bits(SLACK.load(Ordering::Relaxed)) / 2.0
}

pub fn panel_middle_y() -> f32 {
    OUT_H as f32 / 2.0 + raise()
}

pub fn rest_y(h: f32) -> f32 {
    let y = if h > CART_H as f32 {
        (OUT_H as f32 - MOUTH_H - h) / 2.0
    } else {
        (OUT_H + CART_H) as f32 / 2.0 - h
    };
    y - raise()
}

pub fn foot_y(h: f32) -> f32 {
    rest_y(h) + h
}
const OMEGA: f32 = 16.0;
const PART: f32 = 130.0;

const SLOTS: i32 = 3;

/// Gap between a favourite star and the top of its cart, at full cart size.
const STAR_GAP: f32 = 10.0;

const REPEAT_DELAY_MS: Millis = 400;
const REPEAT_MS: [Millis; 4] = [110, 85, 65, 50];

pub struct Shelf {
    pub carts: Vec<Cart>,
    pub index: usize,
    pub scroll: f32,
    faces: Vec<Option<TexId>>,
    shadow: Option<TexId>,
    gb_shadow: Option<TexId>,
    gbc_shadow: Option<TexId>,
    shells: Vec<Option<GbShell>>,
    ride: f32,
    vel: f32,
    held: Option<(i32, Millis, usize)>,
    marks: Vec<bool>,
    star: Option<(TexId, f32, f32)>,
    /// The selected cart's title (Settings > Screen > Show Game Titles): which cart it
    /// was made for, and how far into its marquee it is.
    show_titles: bool,
    title: Option<(usize, ShownTitle)>,
    marquee: f32,
}

impl Shelf {
    pub fn new(carts: Vec<Cart>) -> Self {
        Shelf {
            shells: carts.iter().map(gb_shell_of).collect(),
            faces: vec![None; carts.len()],
            carts,
            index: 0,
            scroll: 0.0,
            shadow: None,
            gb_shadow: None,
            gbc_shadow: None,
            ride: 0.0,
            vel: 0.0,
            held: None,
            marks: Vec::new(),
            star: None,
            show_titles: false,
            title: None,
            marquee: 0.0,
        }
    }

    /// Swaps in a new row of carts, keeping the shadows and the star, and hands back the
    /// faces the old row held so their textures can be reused.
    pub fn restock(&mut self, carts: Vec<Cart>) -> Vec<TexId> {
        let old: Vec<TexId> = self.faces.drain(..).flatten().collect();
        self.shells = carts.iter().map(gb_shell_of).collect();
        self.faces = vec![None; carts.len()];
        self.marks = Vec::new();
        self.carts = carts;
        self.held = None;
        self.select(self.index.min(self.carts.len().saturating_sub(1)));
        old
    }

    pub fn select(&mut self, i: usize) {
        self.index = i;
        self.scroll = i as f32;
        self.ride = i as f32;
        self.vel = 0.0;
    }

    pub fn set_shadow(&mut self, face: TexId) {
        self.shadow = Some(face);
    }

    pub fn set_gb_shadow(&mut self, shell: GbShell, face: TexId) {
        match shell {
            GbShell::Notched => self.gb_shadow = Some(face),
            GbShell::Rounded => self.gbc_shadow = Some(face),
        }
    }

    /// The texture drawn above every marked cart, and its size at full cart scale.
    pub fn set_star(&mut self, tex: TexId, w: u32, h: u32) {
        self.star = Some((tex, w as f32, h as f32));
    }

    /// Which carts wear the favourite star, by index. Missing entries are unmarked.
    pub fn set_marks(&mut self, marks: Vec<bool>) {
        self.marks = marks;
    }

    pub fn is_marked(&self, i: usize) -> bool {
        self.marks.get(i).copied().unwrap_or(false)
    }

    pub fn set_faces(&mut self, faces: Vec<TexId>) {
        let mut faces = faces.into_iter();
        for face in &mut self.faces {
            *face = faces.next();
        }
    }

    pub fn face(&self, i: usize) -> Option<TexId> {
        self.faces.get(i).copied().flatten()
    }

    pub fn set_face(&mut self, i: usize, tex: TexId) -> Option<TexId> {
        self.faces.get_mut(i).and_then(|f| f.replace(tex))
    }

    pub fn take_face(&mut self, i: usize) -> Option<TexId> {
        self.faces.get_mut(i).and_then(Option::take)
    }

    pub fn set_show_titles(&mut self, on: bool) {
        self.show_titles = on;
    }

    pub fn shows_titles(&self) -> bool {
        self.show_titles
    }

    /// The title drawn, if it belongs to the selected cart. One made for a cart that is no
    /// longer selected is never drawn, so fast scrolling cannot show a stale name.
    pub fn title(&self) -> Option<ShownTitle> {
        self.title
            .filter(|(i, _)| *i == self.index && self.show_titles)
            .map(|(_, t)| t)
    }

    /// The selected cart, when titles are on and its title is not ready yet.
    pub fn wants_title(&self) -> Option<usize> {
        (self.show_titles && !self.carts.is_empty() && self.title().is_none()).then_some(self.index)
    }

    /// Places cart `i`'s title, handing back the textures of the one it replaces (or this
    /// one's, if `i` is no longer selected).
    pub fn set_title(&mut self, i: usize, title: ShownTitle) -> Vec<TexId> {
        if i != self.index || i >= self.carts.len() || !self.show_titles {
            return title.textures().collect();
        }
        self.marquee = 0.0;
        self.title
            .replace((i, title))
            .map(|(_, old)| old.textures().collect())
            .unwrap_or_default()
    }

    /// The title's textures, emptied out for reuse.
    pub fn take_titles(&mut self) -> Vec<TexId> {
        self.title
            .take()
            .map(|(_, t)| t.textures().collect())
            .unwrap_or_default()
    }

    /// Whether the title held belongs to a cart that is no longer selected.
    pub fn title_is_stale(&self) -> bool {
        self.title.is_some_and(|(i, _)| i != self.index)
            || (self.title.is_some() && !self.show_titles)
    }

    /// Seconds into the selected title's marquee, set by the app each frame.
    pub fn set_marquee(&mut self, t: f32) {
        self.marquee = t;
    }

    pub fn marquee(&self) -> f32 {
        self.marquee
    }

    pub fn on_screen(&self) -> Vec<usize> {
        self.around(SLOTS)
    }

    pub fn around(&self, radius: i32) -> Vec<usize> {
        let mut out = Vec::new();
        for step in 0..=radius {
            for off in [step, -step] {
                if let Some(i) = self.cart_at_offset(off) {
                    if !out.contains(&i) {
                        out.push(i);
                    }
                }
            }
        }
        out
    }

    pub fn find(&self, stem: &str) -> Option<(&Cart, Option<TexId>)> {
        let i = self.carts.iter().position(|c| c.stem == stem)?;
        Some((&self.carts[i], self.face(i)))
    }

    pub fn left(&mut self) {
        self.step(-1);
    }

    pub fn right(&mut self) {
        self.step(1);
    }

    pub fn jump_next_letter(&mut self) {
        self.jump(1);
    }

    pub fn jump_prev_letter(&mut self) {
        self.jump(-1);
    }

    fn start_of_letter(&self, from: usize) -> usize {
        let n = self.carts.len();
        let letter = slot_store::initial(&self.carts[from].stem);
        let mut at = from;
        for _ in 0..n {
            let before = (at as i32 - 1).rem_euclid(n as i32) as usize;
            if slot_store::initial(&self.carts[before].stem) != letter {
                break;
            }
            at = before;
        }
        at
    }

    fn jump(&mut self, dir: i32) {
        let n = self.carts.len();
        if n < 2 {
            return;
        }
        let wrap = |i: i32| i.rem_euclid(n as i32) as usize;
        let here = slot_store::initial(&self.carts[self.index].stem);
        if self
            .carts
            .iter()
            .all(|c| slot_store::initial(&c.stem) == here)
        {
            return;
        }
        let target = match dir > 0 {
            true => {
                let mut at = self.index;
                for _ in 0..n {
                    at = wrap(at as i32 + 1);
                    if slot_store::initial(&self.carts[at].stem) != here {
                        break;
                    }
                }
                at
            }
            false => {
                let start = self.start_of_letter(self.index);
                match start == self.index {
                    true => self.start_of_letter(wrap(start as i32 - 1)),
                    false => start,
                }
            }
        };
        let ahead = (target as i32 - self.index as i32).rem_euclid(n as i32);
        let delta = match dir > 0 {
            true => ahead,
            false => ahead - n as i32,
        };
        self.index = target;
        self.ride += delta as f32;
    }

    pub fn hold_left(&mut self, now: Millis) {
        self.hold(-1, now);
    }

    pub fn hold_right(&mut self, now: Millis) {
        self.hold(1, now);
    }

    fn hold(&mut self, by: i32, now: Millis) {
        self.step(by);
        self.held = Some((by, now + REPEAT_DELAY_MS, 0));
    }

    pub fn release_left(&mut self) {
        self.release(-1);
    }

    pub fn release_right(&mut self) {
        self.release(1);
    }

    fn release(&mut self, by: i32) {
        if matches!(self.held, Some((held, _, _)) if held == by) {
            self.held = None;
        }
    }

    pub fn release_hold(&mut self) {
        self.held = None;
    }

    pub fn tick(&mut self, now: Millis) {
        let Some((by, due, fired)) = self.held else {
            return;
        };
        if now < due {
            return;
        }
        self.step(by);
        let rate = REPEAT_MS[fired.min(REPEAT_MS.len() - 1)];
        self.held = Some((by, now + rate, fired + 1));
    }

    fn step(&mut self, by: i32) {
        let n = self.carts.len();
        if n < 2 {
            return;
        }
        self.index = (self.index as i32 + by).rem_euclid(n as i32) as usize;
        self.ride += by as f32;
    }

    pub fn scroll_target(&self) -> f32 {
        let n = self.carts.len();
        if n == 0 {
            return 0.0;
        }
        let from = self.ride;
        let n = n as f32;
        from + (self.index as f32 - from + n / 2.0).rem_euclid(n) - n / 2.0
    }

    pub fn cart_at_offset(&self, off: i32) -> Option<usize> {
        let n = self.carts.len() as i32;
        if n == 0 {
            return None;
        }
        if n == 1 {
            return (off == 0).then_some(self.index);
        }
        Some((self.index as i32 + off).rem_euclid(n) as usize)
    }

    pub fn selected_at(&self) -> (f32, f32) {
        let w = self
            .carts
            .get(self.index)
            .map_or(CART_W, |c| cart_box(c.platform).0) as f32;
        let offset = self.scroll_target() - self.scroll;
        let scale = shrink(offset);
        (OUT_W as f32 / 2.0 + offset * w - w * scale / 2.0, scale)
    }

    pub fn update(&mut self, dt: f32) {
        let accel = -2.0 * OMEGA * self.vel - OMEGA * OMEGA * (self.scroll - self.scroll_target());
        self.vel += accel * dt;
        self.scroll += self.vel * dt;
    }

    pub fn draw(&self, shake: f32, out: &mut Vec<Draw>) {
        self.draw_row(None, shake, 0.0, 1.0, out);
        draw_empty_slot(out);
    }

    pub fn draw_row(
        &self,
        hidden: Option<&str>,
        shake: f32,
        recede: f32,
        dim: f32,
        out: &mut Vec<Draw>,
    ) {
        let recede = recede.clamp(0.0, 1.0);
        let dim = dim.clamp(0.0, 1.0);
        let target = self.scroll_target();
        for slot in -SLOTS..=SLOTS {
            let Some(i) = self.cart_at_offset(slot) else {
                continue;
            };
            let cart = &self.carts[i];
            if hidden == Some(cart.stem.as_str()) {
                continue;
            }
            let offset = target + slot as f32 - self.scroll;
            let t = offset.abs().min(1.0);
            let scale = shrink(offset);
            let alpha = (1.0 + (SIDE_ALPHA - 1.0) * t) * (1.0 - recede);
            let (cw, ch) = cart_box(cart.platform);
            let (w, h) = (cw as f32 * scale, ch as f32 * scale);
            let away = offset.signum() * (1.0 + offset.abs());
            let x = OUT_W as f32 / 2.0 + offset * cw as f32 - w / 2.0 + away * PART * recede;
            if x + w <= 0.0 || x >= OUT_W as f32 || alpha <= 0.0 {
                continue;
            }
            let x = x + shake;
            let y = foot_y(ch as f32) - h;
            if alpha < 1.0 {
                let backing = match self.shells.get(i).copied().flatten() {
                    None => self.shadow,
                    Some(GbShell::Notched) => self.gb_shadow,
                    Some(GbShell::Rounded) => self.gbc_shadow,
                };
                if let Some(tex) = backing {
                    out.push(Draw::Tex {
                        x,
                        y,
                        w,
                        h,
                        tex,
                        alpha: recede_alpha(alpha),
                    });
                }
            }
            // With titles on, the selected cart's title sits just above it, and every star
            // sits above the title's space, so stars stay level and never jump.
            let mut top = y;
            if self.show_titles {
                top = y - (TITLE_GAP + TITLE_FACE_H as f32) * scale;
                if i == self.index {
                    if let Some(title) = self.title() {
                        title.draw(x + w / 2.0, y, scale, alpha * dim, self.marquee, out);
                    }
                }
            }
            if let (true, Some((tex, sw, sh))) = (self.is_marked(i), self.star) {
                let (sw, sh) = (sw * scale, sh * scale);
                out.push(Draw::Tex {
                    x: x + (w - sw) / 2.0,
                    y: top - STAR_GAP * scale - sh,
                    w: sw,
                    h: sh,
                    tex,
                    alpha: alpha * dim,
                });
            }
            out.push(match self.face(i) {
                Some(tex) => Draw::Tex {
                    x,
                    y,
                    w,
                    h,
                    tex,
                    alpha: alpha * dim,
                },
                None => {
                    let c = label_colour(&label_text(cart));
                    Draw::Rect {
                        x,
                        y,
                        w,
                        h,
                        colour: [
                            c[0] as f32 / 255.0,
                            c[1] as f32 / 255.0,
                            c[2] as f32 / 255.0,
                            alpha * dim,
                        ],
                    }
                }
            });
        }
    }
}

fn shrink(offset: f32) -> f32 {
    1.0 + (SIDE_SCALE - 1.0) * offset.abs().min(1.0)
}

fn recede_alpha(face_alpha: f32) -> f32 {
    (face_alpha / SIDE_ALPHA).clamp(0.0, 1.0)
}
