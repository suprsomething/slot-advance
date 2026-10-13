//! Free and total space on the filesystem holding a path: for the About screen, the games
//! card (TF2).

use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use slot_ui::CardSpace;

/// Space on the filesystem holding `path`. Free space is what an ordinary user can write
/// (`f_bavail`), as `df` reports it. `None` if the path can't be queried.
// The statvfs fields are u64 on 64-bit Linux but narrower elsewhere, so the casts stay.
#[allow(clippy::unnecessary_cast)]
pub fn card_space(path: &Path) -> Option<CardSpace> {
    let c = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut st = std::mem::MaybeUninit::<libc::statvfs>::zeroed();
    // SAFETY: `c` is a valid NUL-terminated path and `st` points to a statvfs-sized buffer
    // the call fills in. It is only read after the call reports success.
    let rc = unsafe { libc::statvfs(c.as_ptr(), st.as_mut_ptr()) };
    if rc != 0 {
        return None;
    }
    // SAFETY: statvfs returned 0, so it filled `st` in.
    let st = unsafe { st.assume_init() };
    let unit = if st.f_frsize > 0 {
        st.f_frsize as u64
    } else {
        st.f_bsize as u64
    };
    let total = (st.f_blocks as u64).checked_mul(unit)?;
    let free = (st.f_bavail as u64).checked_mul(unit)?;
    (total > 0).then_some(CardSpace {
        free: free.min(total),
        total,
    })
}
