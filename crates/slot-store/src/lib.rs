mod atomic;
pub mod cart_shell;
mod config;
mod core;
mod favourites;
mod game_profile;
pub mod gb;
mod gb_palette;
mod gba;
pub mod ini;
mod platform;
mod recents;
mod ring;
mod scan;
mod slot_state;
mod stamp;
mod theme;

pub use atomic::atomic_write;
pub use cart_shell::{Outline, ShellChoice, ShellFinish, CART_SHELL_FILE, LABELS_SHELL_FILE};
pub use config::{move_config, CONFIG_DIR};
pub use core::{
    core_for, core_for_platform, read_selected_cores, write_selected_core, Core, SELECTED_CORE_FILE,
};
pub use favourites::{favourite_key, read_favourites, write_favourites, FAVOURITES_FILE};
pub use game_profile::{
    profile_key, GameProfile, GameProfiles, ProfileField, Resolved, GAME_PROFILES_FILE,
};
pub use gb_palette::GbPalette;
pub use gba::{header_clean, header_code, header_title, header_title_code};
pub use platform::Platform;
pub use recents::{read_recents, touch_recent, write_recents, RECENTS_FILE, RECENTS_MAX};
pub use ring::{StateEntry, StateRing, OLDER_DIR, RING_MAX};
pub use scan::{initial, is_hidden, listing, scan, sort_key, Cart, StoreError};
pub use slot_state::{
    read_slot_state, write_slot_state, Category, Shader, SlotState, BLUE_LIGHT_MAX, BRIGHTNESS_MAX,
    FF_SPEEDS, FF_SPEED_DEFAULT, SLEEP_MINUTES, SLEEP_MINUTES_DEFAULT, UTC_OFFSET_MAX,
    UTC_OFFSET_MIN, VOLUME_MAX,
};
pub use stamp::{
    civil_from_days, days_from_civil, days_in_month, format_stamp, parse_stamp, stamp_now,
};
pub use theme::{Theme, THEME_FILE};
