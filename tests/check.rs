extern crate integrity_checker;

use std::path::{Path, PathBuf};

use integrity_checker::database::{Database, DiffSummary};

fn check(root_dir: impl AsRef<Path>) -> DiffSummary {
    let mut before_path = PathBuf::from(root_dir.as_ref());
    before_path.push("before");

    let mut after_path = PathBuf::from(root_dir.as_ref());
    after_path.push("after");

    let threads = 1;
    let before_db = Database::build(&before_path, false, threads).unwrap();
    before_db.check(&after_path, threads).unwrap()
}

#[test]
fn no_changes() {
    let result = check("tests/nochanges");
    assert_eq!(result, DiffSummary::NoChanges);
}

#[test]
fn changes_edit() {
    let result = check("tests/changes_edit");
    assert_eq!(result, DiffSummary::Changes);
}

#[test]
fn changes_new() {
    let result = check("tests/changes_new");
    assert_eq!(result, DiffSummary::Changes);
}

#[test]
fn changes_edit_bin() {
    let result = check("tests/changes_edit_bin");
    assert_eq!(result, DiffSummary::Changes);
}

#[test]
fn changes_new_bin() {
    let result = check("tests/changes_new_bin");
    assert_eq!(result, DiffSummary::Changes);
}

#[test]
fn changes_delete() {
    let result = check("tests/changes_delete");
    assert_eq!(result, DiffSummary::Changes);
}

#[test]
fn suspicious_truncate() {
    let result = check("tests/suspicious_truncate");
    assert_eq!(result, DiffSummary::Suspicious);
}

#[test]
fn suspicious_nul() {
    let result = check("tests/suspicious_nul");
    assert_eq!(result, DiffSummary::Suspicious);
}

#[test]
fn suspicious_nonascii() {
    let result = check("tests/suspicious_nonascii");
    assert_eq!(result, DiffSummary::Suspicious);
}
