mod common;

use common::{app_booting_at, tmp_root_with_carts};
use slot::app::{App, Phase};
use slot_input::{Action, Btn};
use slot_store::{
    read_slot_state, write_slot_state, GameProfiles, Platform, Shader, SlotState,
    GAME_PROFILES_FILE,
};
use tempfile::TempDir;

const CLOCK_IS_SET: i64 = 1_700_000_000;

fn with_defaults(state: SlotState) -> TempDir {
    let d = tmp_root_with_carts(&["Emerald", "Fusion"]);
    write_slot_state(
        d.path(),
        &SlotState {
            clock_set: true,
            ..state
        },
    )
    .expect("write slot.state");
    d
}

fn boot(d: &TempDir) -> App {
    let (a, _clock) = app_booting_at(d.path(), CLOCK_IS_SET);
    assert!(matches!(a.phase(), Phase::Shelf), "{:?}", a.phase());
    a
}

fn settle(a: &mut App) {
    for _ in 0..120 {
        a.update(1.0 / 60.0);
    }
}

fn play(a: &mut App, stem: &str) {
    for _ in 0..4 {
        if a.selected_stem() == Some(stem) {
            break;
        }
        a.apply(Action::GbaDown(Btn::Right));
        a.apply(Action::GbaUp(Btn::Right));
        a.update(1.0);
    }
    assert_eq!(a.selected_stem(), Some(stem));
    a.apply(Action::Insert);
    a.on_core_ready();
    settle(a);
    assert!(
        matches!(a.phase(), Phase::Playing { .. }),
        "{:?}",
        a.phase()
    );
}

fn eject(a: &mut App) {
    a.apply(Action::Eject);
    settle(a);
    assert!(matches!(a.phase(), Phase::Shelf), "{:?}", a.phase());
}

#[test]
fn a_game_without_a_profile_runs_on_the_defaults() {
    let d = with_defaults(SlotState {
        shader_gba: Shader::Grid,
        colour_correction: true,
        ff_speed: 4,
        ff_sound: true,
        ..Default::default()
    });
    let mut a = boot(&d);
    play(&mut a, "Emerald");
    assert_eq!(a.screen_shader(), Shader::Grid);
    assert!(a.colour_correction());
    assert_eq!(a.ff_speed(), 4);
    assert!(a.ff_sound());
    assert!(
        !d.path().join(GAME_PROFILES_FILE).exists(),
        "playing a game wrote a profile nobody asked for"
    );
}

#[test]
fn a_shader_chosen_in_one_game_stays_with_that_game() {
    let d = with_defaults(SlotState::default());
    let mut a = boot(&d);
    play(&mut a, "Emerald");
    a.apply(Action::ShaderNext);
    assert_eq!(a.screen_shader(), Shader::Lcd3x);
    eject(&mut a);
    assert_eq!(
        a.screen_shader(),
        Shader::Off,
        "the carousel kept the last game's shader"
    );

    play(&mut a, "Fusion");
    assert_eq!(
        a.screen_shader(),
        Shader::Off,
        "Emerald's shader leaked into Fusion"
    );
    eject(&mut a);
    play(&mut a, "Emerald");
    assert_eq!(
        a.screen_shader(),
        Shader::Lcd3x,
        "Emerald forgot its shader"
    );
}

#[test]
fn colour_correction_toggled_in_game_is_that_games_alone() {
    let d = with_defaults(SlotState::default());
    let mut a = boot(&d);
    play(&mut a, "Emerald");
    let _ = a.take_colour_correction();
    a.apply(Action::ColourCorrectionToggle);
    assert!(a.colour_correction());
    assert_eq!(
        a.take_colour_correction(),
        Some(true),
        "the running core was not told"
    );
    eject(&mut a);
    play(&mut a, "Fusion");
    assert!(
        !a.colour_correction(),
        "colour correction leaked into Fusion"
    );
    a.settle_settings();
    assert!(
        !read_slot_state(d.path()).colour_correction,
        "an in-game toggle changed the default"
    );
}

#[test]
fn profiles_survive_a_reboot_and_an_update_that_replaces_system() {
    let d = with_defaults(SlotState::default());
    let mut a = boot(&d);
    play(&mut a, "Emerald");
    a.apply(Action::ShaderNext);
    a.apply(Action::ColourCorrectionToggle);
    a.settle_settings();
    drop(a);

    let system = d.path().join("System");
    let _ = std::fs::remove_dir_all(&system);
    std::fs::create_dir_all(&system).unwrap();

    // Emerald was still in the slot, so the reboot resumes it straight away.
    let (mut b, _clock) = app_booting_at(d.path(), CLOCK_IS_SET);
    assert!(
        matches!(b.phase(), Phase::Inserting { cart, .. } if cart == "Emerald"),
        "{:?}",
        b.phase()
    );
    assert_eq!(
        b.screen_shader(),
        Shader::Lcd3x,
        "the resumed game lost its shader"
    );
    assert!(
        b.colour_correction(),
        "the core would open without the game's colour"
    );
    b.on_core_ready();
    settle(&mut b);
    assert_eq!(b.screen_shader(), Shader::Lcd3x);
    let p = GameProfiles::read(d.path()).get(Platform::Gba, "Emerald");
    assert_eq!(p.shader, Some(Shader::Lcd3x));
    assert_eq!(p.colour, Some(true));
    assert_eq!(
        p.ff_speed, None,
        "a setting that was never changed was written"
    );
}

#[test]
fn changing_a_default_moves_games_that_follow_it_but_not_games_that_chose() {
    let d = with_defaults(SlotState::default());
    let mut a = boot(&d);
    play(&mut a, "Emerald");
    a.apply(Action::ShaderNext);
    a.settle_settings();
    eject(&mut a);
    drop(a);

    let mut state = read_slot_state(d.path());
    state.shader_gba = Shader::Dot;
    write_slot_state(d.path(), &state).unwrap();

    let mut b = boot(&d);
    play(&mut b, "Fusion");
    assert_eq!(
        b.screen_shader(),
        Shader::Dot,
        "Fusion ignored the new default"
    );
    eject(&mut b);
    play(&mut b, "Emerald");
    assert_eq!(
        b.screen_shader(),
        Shader::Lcd3x,
        "the new default overrode Emerald's own choice"
    );
}

fn select_r2(a: &mut App) {
    a.apply(Action::PaletteNext);
}

fn select_l2(a: &mut App) {
    a.apply(Action::PalettePrev);
}

#[test]
fn select_l2_and_r2_step_this_games_fast_forward_speed_and_stop_at_the_ends() {
    let d = with_defaults(SlotState {
        ff_speed: 4,
        ..Default::default()
    });
    let mut a = boot(&d);
    play(&mut a, "Emerald");
    assert_eq!(a.ff_speed(), 4);
    select_l2(&mut a);
    assert_eq!(a.ff_speed(), 3);
    assert_eq!(a.toast(), Some(slot_ui::Toast::FastForward3));
    select_l2(&mut a);
    select_l2(&mut a);
    assert_eq!(a.ff_speed(), 2, "the slowest speed wrapped round");
    for _ in 0..5 {
        select_r2(&mut a);
    }
    assert_eq!(a.ff_speed(), 6, "the fastest speed wrapped round");
    assert_eq!(a.toast(), Some(slot_ui::Toast::FastForward6));
    a.settle_settings();
    assert_eq!(
        GameProfiles::read(d.path())
            .get(Platform::Gba, "Emerald")
            .ff_speed,
        Some(6)
    );
    eject(&mut a);
    play(&mut a, "Fusion");
    assert_eq!(a.ff_speed(), 4, "Emerald's speed leaked into Fusion");
    assert_eq!(read_slot_state(d.path()).ff_speed, 4, "the default moved");
}

#[test]
fn select_x_toggles_this_games_fast_forward_sound() {
    let d = with_defaults(SlotState::default());
    let mut a = boot(&d);
    play(&mut a, "Emerald");
    assert!(!a.ff_sound());
    a.apply(Action::FfSoundToggle);
    assert!(a.ff_sound());
    assert_eq!(a.toast(), Some(slot_ui::Toast::FfSoundOn));
    eject(&mut a);
    play(&mut a, "Fusion");
    assert!(
        !a.ff_sound(),
        "Emerald's fast-forward sound leaked into Fusion"
    );
    eject(&mut a);
    play(&mut a, "Emerald");
    assert!(a.ff_sound(), "Emerald forgot its fast-forward sound");
}

#[test]
fn a_game_boy_cart_on_a_named_palette_still_steps_its_palette() {
    let d = with_defaults(SlotState {
        gb_palettes: true,
        ..Default::default()
    });
    let mut a = boot(&d);
    play(&mut a, "Emerald");
    a.set_palette_live(true);
    let speed = a.ff_speed();
    let palette = a.gb_palette();
    select_r2(&mut a);
    assert_eq!(
        a.ff_speed(),
        speed,
        "SELECT + R2 changed fast-forward on a palette cart"
    );
    assert_ne!(a.gb_palette(), palette, "the palette did not step");
}

#[test]
fn fast_forward_chords_do_nothing_on_the_carousel() {
    let d = with_defaults(SlotState::default());
    let mut a = boot(&d);
    let speed = a.ff_speed();
    select_r2(&mut a);
    a.apply(Action::FfSoundToggle);
    assert_eq!(a.ff_speed(), speed);
    assert!(!a.ff_sound());
    a.settle_settings();
    assert!(!d.path().join(GAME_PROFILES_FILE).exists());
}
