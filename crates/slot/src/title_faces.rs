//! Builds the title faces drawn above carts when Settings > Screen > Show Game Titles is
//! on, for the carts about to be on screen, and recycles the textures of carts that have
//! scrolled well away (or all of them when titles are switched off).

use slot_gfx::{Compositor, TexId};
use slot_ui::{cart_box, cart_title_face, display_title, TitleTex, TITLE_INSET};

use crate::app::App;

/// At most this many faces are drawn in one frame: a full row is seven, so a whole
/// shelf's titles arrive on the first frame and scrolling adds one or two at a time.
const PER_FRAME: usize = 7;

#[derive(Default)]
pub struct TitleFaces {
    free: Vec<TexId>,
}

impl TitleFaces {
    pub fn sync(&mut self, app: &mut App, compositor: &mut Compositor) {
        self.free.extend(app.shed_titles());
        for (shelf, index) in app.title_wants().into_iter().take(PER_FRAME) {
            let Some(cart) = app.face_cart(shelf, index) else {
                continue;
            };
            let max_w = cart_box(cart.platform).0.saturating_sub(2 * TITLE_INSET);
            let face = cart_title_face(&display_title(&cart.stem), max_w);
            let tex = match self.free.pop() {
                Some(tex) => {
                    compositor.update_texture(tex, face.w, face.h, &face.rgba);
                    tex
                }
                None => compositor.create_texture(face.w, face.h, &face.rgba),
            };
            let title = TitleTex {
                tex,
                w: face.w as f32,
                h: face.h as f32,
            };
            self.free.extend(app.set_cart_title(shelf, index, title));
        }
    }
}
