use slot_gfx::{Draw, TexId, OUT_H, OUT_W};
use slot_power::Battery;
use slot_store::{parse_stamp, StateEntry};

use crate::art;
use crate::battery::{draw_gauge, GAUGE_H};
use crate::footer::{draw_printed, Printed};
use crate::hud::{PLATE, PLATE_H};
use crate::plate::{hint_quad, hint_row, hint_width, Hint, HINT_GAP, HINT_H, TITLE_H, TITLE_W};

pub const PHOTO_W: u32 = 240;
pub const PHOTO_H: u32 = 160;

const BLANK: [u8; 3] = [0x3a, 0x3a, 0x3e];

pub const DOT: f32 = 6.0;
const DOT_GAP: f32 = 10.0;
const DOT_DIM: f32 = 0.35;

const MARGIN: f32 = 16.0;

pub const LEGEND: [(&str, &str); 3] = [("B", "Back"), ("Y", "Delete"), ("A", "Load")];
const WAYS_OUT: usize = 2;

/// The bottom plate while a delete waits for its second Y: keeping the state on the left,
/// where the ways out sit, and deleting it on the right, under the screenshot's own action.
pub const CONFIRM_LEGEND: [(&str, &str); 2] = [("B", "Keep"), ("Y", "Delete")];
const CONFIRM_WAYS_OUT: usize = 1;

/// What the top plate says in place of the timestamp while a delete waits for its answer.
pub const CONFIRM_TITLE: &str = "Delete this state?";
const UNDO_KEY: &str = "X";

pub struct PhotoFace {
    pub rgba: Vec<u8>,
    pub w: u32,
    pub h: u32,
}

pub fn photo_face(entry: &StateEntry) -> PhotoFace {
    let rgba = art::cover(&entry.thumb, PHOTO_W, PHOTO_H).unwrap_or_else(|| {
        std::iter::repeat_n(
            [BLANK[0], BLANK[1], BLANK[2], 255],
            (PHOTO_W * PHOTO_H) as usize,
        )
        .flatten()
        .collect()
    });
    PhotoFace {
        rgba,
        w: PHOTO_W,
        h: PHOTO_H,
    }
}

pub struct Polaroids {
    pub entries: Vec<StateEntry>,
    pub index: usize,
    faces: Vec<TexId>,
    title: Option<TexId>,
    hint_faces: Vec<TexId>,
    undo: Option<String>,
    confirming: bool,
}

impl Polaroids {
    pub fn new(entries: Vec<StateEntry>) -> Self {
        Polaroids {
            entries,
            index: 0,
            faces: Vec::new(),
            title: None,
            hint_faces: Vec::new(),
            undo: None,
            confirming: false,
        }
    }

    /// The first Y: ask before deleting the state on screen.
    pub fn ask_delete(&mut self) {
        self.confirming = !self.entries.is_empty();
    }

    /// Anything but a second Y: keep the state.
    pub fn keep(&mut self) {
        self.confirming = false;
    }

    pub fn is_confirming(&self) -> bool {
        self.confirming
    }

    pub fn set_faces(&mut self, faces: Vec<TexId>) {
        self.faces = faces;
    }

    pub fn set_title_face(&mut self, face: Option<TexId>) {
        self.title = face;
    }

    pub fn set_hint_faces(&mut self, faces: Vec<TexId>) {
        self.hint_faces = faces;
    }

    pub fn set_undo(&mut self, label: Option<&str>) {
        self.undo = label.map(str::to_string);
    }

    pub fn hints(&self) -> Vec<Hint> {
        if self.confirming {
            return hint_row(&CONFIRM_LEGEND);
        }
        let mut out = hint_row(&LEGEND);
        if let Some(label) = &self.undo {
            out.push(Hint {
                key: UNDO_KEY,
                label: label.clone(),
            });
        }
        out
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn selected(&self) -> Option<&StateEntry> {
        self.entries.get(self.index)
    }

    pub fn dots(&self) -> (usize, usize) {
        (self.entries.len(), self.index)
    }

    pub fn title(&self, now: &str) -> String {
        if self.confirming {
            return CONFIRM_TITLE.to_string();
        }
        match self.selected() {
            Some(e) => Self::relative_time(&e.stamp, now),
            None => String::new(),
        }
    }

    pub fn remove_selected(&mut self) {
        if self.index >= self.entries.len() {
            return;
        }
        self.confirming = false;
        self.entries.remove(self.index);
        if self.index < self.faces.len() {
            self.faces.remove(self.index);
        }
        self.index = self.index.min(self.entries.len().saturating_sub(1));
    }

    pub fn left(&mut self) {
        self.index = self.index.saturating_sub(1);
    }

    pub fn right(&mut self) {
        self.index = (self.index + 1).min(self.entries.len().saturating_sub(1));
    }

    pub fn draw(
        &self,
        battery: Option<Battery>,
        battery_percent: Printed,
        bolt: Option<TexId>,
        clock: Printed,
        out: &mut Vec<Draw>,
    ) {
        self.draw_photo(out);
        self.draw_top(battery, battery_percent, bolt, clock, out);
        self.draw_bottom(out);
    }

    fn draw_photo(&self, out: &mut Vec<Draw>) {
        let (w, h) = (OUT_W as f32, OUT_H as f32);
        out.push(match self.faces.get(self.index) {
            Some(tex) => Draw::Shot { tex: *tex },
            None => Draw::Rect {
                x: 0.0,
                y: 0.0,
                w,
                h,
                colour: [
                    BLANK[0] as f32 / 255.0,
                    BLANK[1] as f32 / 255.0,
                    BLANK[2] as f32 / 255.0,
                    1.0,
                ],
            },
        });
    }

    fn draw_top(
        &self,
        battery: Option<Battery>,
        battery_percent: Printed,
        bolt: Option<TexId>,
        clock: Printed,
        out: &mut Vec<Draw>,
    ) {
        out.push(plate(0.0));
        if let Some(tex) = self.title {
            out.push(Draw::Tex {
                x: (OUT_W as f32 - TITLE_W as f32) / 2.0,
                y: (PLATE_H - TITLE_H as f32) / 2.0,
                w: TITLE_W as f32,
                h: TITLE_H as f32,
                tex,
                alpha: 1.0,
            });
        }

        draw_gauge(
            MARGIN,
            (PLATE_H - GAUGE_H) / 2.0,
            battery,
            battery_percent,
            bolt,
            out,
        );
        if clock.w > 0 {
            let x = OUT_W as f32 - MARGIN - clock.w as f32;
            draw_printed(x, (PLATE_H - HINT_H as f32) / 2.0, clock, out);
        }
    }

    fn draw_bottom(&self, out: &mut Vec<Draw>) {
        let top = OUT_H as f32 - PLATE_H;
        out.push(plate(top));
        let y = top + (PLATE_H - HINT_H as f32) / 2.0;
        let hints = self.hints();
        let widths: Vec<f32> = hints
            .iter()
            .map(|h| hint_width(h.key, &h.label) as f32)
            .collect();
        let ways_out = match self.confirming {
            true => CONFIRM_WAYS_OUT,
            false => WAYS_OUT,
        };
        let tail = &widths[ways_out..];
        let tail_w = tail.iter().sum::<f32>() + HINT_GAP * (tail.len() - 1) as f32;
        let mut x = MARGIN;
        for (i, w) in widths.iter().enumerate() {
            if i == ways_out {
                x = OUT_W as f32 - MARGIN - tail_w;
            }
            out.push(hint_quad(x, y, *w, self.hint_faces.get(i).copied()));
            x += w + HINT_GAP;
        }
        self.draw_dots(top, out);
    }

    fn draw_dots(&self, top: f32, out: &mut Vec<Draw>) {
        let n = self.entries.len();
        let row = n as f32 * DOT + (n as f32 - 1.0).max(0.0) * DOT_GAP;
        let mut x = (OUT_W as f32 - row) / 2.0;
        for i in 0..n {
            out.push(Draw::Rect {
                x,
                y: top + (PLATE_H - DOT) / 2.0,
                w: DOT,
                h: DOT,
                colour: [1.0, 1.0, 1.0, if i == self.index { 1.0 } else { DOT_DIM }],
            });
            x += DOT + DOT_GAP;
        }
    }

    pub fn relative_time(stamp: &str, now: &str) -> String {
        const RELATIVE_WINDOW: i64 = 12 * 3600;
        let (Some(then), Some(parsed)) = (parse_stamp(stamp), parse_stamp(now)) else {
            return stamp.to_string();
        };
        let delta = parsed - then;
        if delta < 60 {
            return "just now".into();
        }
        if delta < 3600 {
            return format!("{} min ago", delta / 60);
        }
        if delta < RELATIVE_WINDOW {
            return format!("{} hr ago", delta / 3600);
        }
        format!("{} {}:{}", &stamp[..10], &stamp[11..13], &stamp[14..16])
    }
}

fn plate(y: f32) -> Draw {
    Draw::Rect {
        x: 0.0,
        y,
        w: OUT_W as f32,
        h: PLATE_H,
        colour: PLATE,
    }
}
