//! Builds the selected cart's title when Settings > Screen > Show Game Titles is on: a
//! face that fits the title's window and, for a long title, the whole line to scroll. Both
//! are drawn once per selection, never per frame, into textures that are reused.

use slot_gfx::{Compositor, TexId};
use slot_ui::{display_title, title_faces, ShownTitle, TitleTex, UndoFace};

use crate::app::App;

#[derive(Default)]
pub struct TitleFaces {
    free: Vec<TexId>,
}

impl TitleFaces {
    pub fn sync(&mut self, app: &mut App, compositor: &mut Compositor) {
        self.free.extend(app.shed_titles());
        let Some((shelf, index)) = app.title_wants() else {
            return;
        };
        let Some(cart) = app.face_cart(shelf, index) else {
            return;
        };
        let faces = title_faces(&display_title(&cart.stem));
        let short = self.upload(compositor, &faces.short);
        let full = faces.full.as_ref().map(|f| self.upload(compositor, f));
        let title = ShownTitle { short, full };
        self.free.extend(app.set_cart_title(shelf, index, title));
    }

    fn upload(&mut self, compositor: &mut Compositor, face: &UndoFace) -> TitleTex {
        let tex = match self.free.pop() {
            Some(tex) => {
                compositor.update_texture(tex, face.w, face.h, &face.rgba);
                tex
            }
            None => compositor.create_texture(face.w, face.h, &face.rgba),
        };
        TitleTex {
            tex,
            w: face.w as f32,
            h: face.h as f32,
        }
    }
}
