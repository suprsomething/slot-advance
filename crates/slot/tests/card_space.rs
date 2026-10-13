use slot::card_space::card_space;

#[test]
fn a_real_folder_reports_its_filesystems_space() {
    let dir = tempfile::tempdir().unwrap();
    let space = card_space(dir.path()).expect("a temp folder can be queried");
    assert!(space.total > 0);
    assert!(space.free <= space.total);
}

#[test]
fn a_missing_folder_reports_nothing_rather_than_zero() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(card_space(&dir.path().join("not-there")), None);
}
