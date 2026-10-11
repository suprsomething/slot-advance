use slot::build_info::Build;

fn sample() -> Build {
    Build {
        version: "0.1.0",
        hash: "9e11a10",
        dirty: false,
        date: "2026-08-12",
    }
}

#[test]
fn the_serial_is_upper_case_hex() {
    assert_eq!(sample().serial(), "9E11A10");
    assert!(sample().serial().chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn the_boxed_digit_says_whether_the_tree_was_dirty() {
    let dirty = Build {
        dirty: true,
        ..sample()
    };
    assert_eq!(sample().dirty_digit(), '0');
    assert_eq!(dirty.dirty_digit(), '1');
}

#[test]
fn a_build_with_no_git_still_has_a_serial() {
    let unknown = Build {
        hash: "unknown",
        date: "unknown",
        ..sample()
    };
    assert!(!unknown.serial().is_empty());
    let payload = format!("*SLOT-{}-{}*", unknown.serial(), unknown.dirty_digit());
    assert!(slot_ui::code39(&payload).is_some());
}

#[test]
fn the_current_build_is_populated() {
    let b = Build::current();
    assert!(!b.version.is_empty());
    assert!(!b.hash.is_empty());
    assert!(!b.date.is_empty());
}

#[test]
fn the_log_line_names_the_version_and_commit() {
    let line = Build {
        version: "0.4.0-beta.1",
        ..sample()
    }
    .describe();
    assert_eq!(
        line,
        "slot advance 0.4.0-beta.1 (9e11a10, built 2026-08-12)"
    );
    let dirty = Build {
        dirty: true,
        ..sample()
    };
    assert!(
        dirty.describe().contains("9e11a10+changes"),
        "{}",
        dirty.describe()
    );
}

/// The version is semantic: MAJOR.MINOR.PATCH with an optional pre-release such as
/// "-beta.1".
#[test]
fn the_version_is_semantic() {
    let v = Build::current().version;
    let (core, pre) = v.split_once('-').unwrap_or((v, ""));
    let parts: Vec<&str> = core.split('.').collect();
    assert_eq!(parts.len(), 3, "{v}");
    assert!(
        parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())),
        "{v}"
    );
    assert!(
        pre.is_empty() || pre.split('.').all(|p| !p.is_empty()),
        "{v}"
    );
}

/// The README states the version this code builds as, so the two cannot drift apart.
#[test]
fn the_readme_states_the_current_version() {
    let readme = include_str!("../../../README.md");
    let v = Build::current().version;
    assert!(
        readme.contains(&format!("This build: **{v}**")),
        "README.md does not say \"This build: **{v}**\""
    );
}
