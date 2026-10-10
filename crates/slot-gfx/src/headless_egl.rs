//! Off-screen GLES 2 context for render tests on Linux, through EGL pbuffers.
//!
//! The macOS `HeadlessSurface` drives desktop GL through CGL. This one asks EGL for an
//! OpenGL ES 2 context, the same API the device's fbdev surface uses, so a render test on
//! Linux (Mesa's software rasteriser is enough) exercises the shaders the way the RG SP
//! compiles them. Set `EGL_PLATFORM=surfaceless` when no display server is running.
//!
//! `window_size` is 720x480 like the macOS surface, or `SLOT_HEADLESS_PANEL=WxH` to
//! render at a real panel's size, eg `640x480` for the RG SP.

use std::ffi::{c_char, c_void, CString};

use libloading::Library;

use crate::surface::{GfxError, Surface, OUT_H, OUT_W};

const EGL_NONE: i32 = 0x3038;
const EGL_ALPHA_SIZE: i32 = 0x3021;
const EGL_BLUE_SIZE: i32 = 0x3022;
const EGL_GREEN_SIZE: i32 = 0x3023;
const EGL_RED_SIZE: i32 = 0x3024;
const EGL_SURFACE_TYPE: i32 = 0x3033;
const EGL_HEIGHT: i32 = 0x3056;
const EGL_WIDTH: i32 = 0x3057;
const EGL_RENDERABLE_TYPE: i32 = 0x3040;
const EGL_CONTEXT_CLIENT_VERSION: i32 = 0x3098;
const EGL_OPENGL_ES_API: u32 = 0x30A0;
const EGL_PBUFFER_BIT: i32 = 0x0001;
const EGL_OPENGL_ES2_BIT: i32 = 0x0004;

type Ptr = *mut c_void;

pub struct HeadlessSurface {
    egl: Library,
    _gles: Library,
    display: Ptr,
    surface: Ptr,
    context: Ptr,
    size: (u32, u32),
}

fn open(name: &str) -> Result<Library, GfxError> {
    unsafe { Library::new(name) }.map_err(|e| GfxError::Context(format!("{name}: {e}")))
}

unsafe fn sym<T: Copy>(lib: &Library, name: &str) -> Result<T, GfxError> {
    lib.get::<T>(name.as_bytes())
        .map(|s| *s)
        .map_err(|e| GfxError::Context(format!("{name}: {e}")))
}

fn panel() -> (u32, u32) {
    std::env::var("SLOT_HEADLESS_PANEL")
        .ok()
        .and_then(|v| {
            let (w, h) = v.split_once('x')?;
            Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
        })
        .filter(|(w, h): &(u32, u32)| *w > 0 && *h > 0)
        .unwrap_or((OUT_W, OUT_H))
}

impl HeadlessSurface {
    pub fn new() -> Result<Self, GfxError> {
        let egl = open("libEGL.so.1").or_else(|_| open("libEGL.so"))?;
        let gles = open("libGLESv2.so.2").or_else(|_| open("libGLESv2.so"))?;
        let size = panel();
        unsafe {
            let get_display: unsafe extern "C" fn(Ptr) -> Ptr = sym(&egl, "eglGetDisplay")?;
            let initialize: unsafe extern "C" fn(Ptr, *mut i32, *mut i32) -> u32 =
                sym(&egl, "eglInitialize")?;
            let bind_api: unsafe extern "C" fn(u32) -> u32 = sym(&egl, "eglBindAPI")?;
            let choose_config: unsafe extern "C" fn(
                Ptr,
                *const i32,
                *mut Ptr,
                i32,
                *mut i32,
            ) -> u32 = sym(&egl, "eglChooseConfig")?;
            let create_pbuffer: unsafe extern "C" fn(Ptr, Ptr, *const i32) -> Ptr =
                sym(&egl, "eglCreatePbufferSurface")?;
            let create_context: unsafe extern "C" fn(Ptr, Ptr, Ptr, *const i32) -> Ptr =
                sym(&egl, "eglCreateContext")?;
            let make_current: unsafe extern "C" fn(Ptr, Ptr, Ptr, Ptr) -> u32 =
                sym(&egl, "eglMakeCurrent")?;

            let display = get_display(std::ptr::null_mut());
            if display.is_null()
                || initialize(display, std::ptr::null_mut(), std::ptr::null_mut()) == 0
            {
                return Err(GfxError::Context("egl: no display".into()));
            }
            if bind_api(EGL_OPENGL_ES_API) == 0 {
                return Err(GfxError::Context("egl: no gles api".into()));
            }
            let attrs = [
                EGL_SURFACE_TYPE,
                EGL_PBUFFER_BIT,
                EGL_RED_SIZE,
                8,
                EGL_GREEN_SIZE,
                8,
                EGL_BLUE_SIZE,
                8,
                EGL_ALPHA_SIZE,
                8,
                EGL_RENDERABLE_TYPE,
                EGL_OPENGL_ES2_BIT,
                EGL_NONE,
            ];
            let mut config: Ptr = std::ptr::null_mut();
            let mut found = 0;
            if choose_config(display, attrs.as_ptr(), &mut config, 1, &mut found) == 0 || found == 0
            {
                return Err(GfxError::Context("egl: no es2 pbuffer config".into()));
            }
            let pb = [
                EGL_WIDTH,
                size.0 as i32,
                EGL_HEIGHT,
                size.1 as i32,
                EGL_NONE,
            ];
            let surface = create_pbuffer(display, config, pb.as_ptr());
            if surface.is_null() {
                return Err(GfxError::Context("egl: no pbuffer".into()));
            }
            let ctx = [EGL_CONTEXT_CLIENT_VERSION, 2, EGL_NONE];
            let context = create_context(display, config, std::ptr::null_mut(), ctx.as_ptr());
            if context.is_null() {
                return Err(GfxError::Context("egl: no es2 context".into()));
            }
            if make_current(display, surface, surface, context) == 0 {
                return Err(GfxError::Context("egl: make current".into()));
            }
            Ok(HeadlessSurface {
                egl,
                _gles: gles,
                display,
                surface,
                context,
                size,
            })
        }
    }
}

impl Surface for HeadlessSurface {
    fn make_current(&mut self) -> Result<(), GfxError> {
        unsafe {
            let make_current: unsafe extern "C" fn(Ptr, Ptr, Ptr, Ptr) -> u32 =
                sym(&self.egl, "eglMakeCurrent")?;
            match make_current(self.display, self.surface, self.surface, self.context) {
                0 => Err(GfxError::Context("egl: make current".into())),
                _ => Ok(()),
            }
        }
    }

    fn window_size(&self) -> (u32, u32) {
        self.size
    }

    fn swap(&mut self) -> Result<(), GfxError> {
        unsafe { gl::Flush() };
        Ok(())
    }

    fn proc_address(&self, name: &str) -> *const c_void {
        let Ok(sym_name) = CString::new(name) else {
            return std::ptr::null();
        };
        unsafe {
            let Ok(get): Result<unsafe extern "C" fn(*const c_char) -> *const c_void, _> =
                sym(&self.egl, "eglGetProcAddress")
            else {
                return std::ptr::null();
            };
            get(sym_name.as_ptr())
        }
    }
}

impl Drop for HeadlessSurface {
    fn drop(&mut self) {
        unsafe {
            if let Ok(make_current) =
                sym::<unsafe extern "C" fn(Ptr, Ptr, Ptr, Ptr) -> u32>(&self.egl, "eglMakeCurrent")
            {
                make_current(
                    self.display,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
            }
            if let Ok(destroy_ctx) =
                sym::<unsafe extern "C" fn(Ptr, Ptr) -> u32>(&self.egl, "eglDestroyContext")
            {
                destroy_ctx(self.display, self.context);
            }
            if let Ok(destroy_surface) =
                sym::<unsafe extern "C" fn(Ptr, Ptr) -> u32>(&self.egl, "eglDestroySurface")
            {
                destroy_surface(self.display, self.surface);
            }
        }
    }
}
