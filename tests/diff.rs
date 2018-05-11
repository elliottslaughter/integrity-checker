extern crate integrity_checker;

use std::path::{Path, PathBuf};

use integrity_checker::database::{Database, DiffSummary};

fn diff(root_dir: impl AsRef<Path>) -> DiffSummary {
    let mut before_path = PathBuf::from(root_dir.as_ref());
    before_path.push("before");

    let mut after_path = PathBuf::from(root_dir.as_ref());
    after_path.push("after");

    let threads = 1;
    let before_db = Database::build(&before_path, false, threads).unwrap();
    let after_db = Database::build(&after_path, false, threads).unwrap();
    before_db.show_diff(&after_db)
}

#[test]
fn no_changes() {
    let result = diff("tests/nochanges");
    assert_eq!(result, DiffSummary::NoChanges);
}

#[test]
fn changes_edit() {
    let result = diff("tests/changes_edit");
    assert_eq!(result, DiffSummary::Changes);
}

#[test]
fn changes_new() {
    let result = diff("tests/changes_new");
    assert_eq!(result, DiffSummary::Changes);
}

#[test]
fn changes_edit_bin() {
    let result = diff("tests/changes_edit_bin");
    assert_eq!(result, DiffSummary::Changes);
}

#[test]
fn changes_new_bin() {
    let result = diff("tests/changes_new_bin");
    assert_eq!(result, DiffSummary::Changes);
}

#[test]
fn changes_delete() {
    let result = diff("tests/changes_delete");
    assert_eq!(result, DiffSummary::Changes);
}

#[test]
fn suspicious_truncate() {
    let result = diff("tests/suspicious_truncate");
    assert_eq!(result, DiffSummary::Suspicious);
}

#[test]
fn suspicious_nul() {
    let result = diff("tests/suspicious_nul");
    assert_eq!(result, DiffSummary::Suspicious);
}

#[test]
fn suspicious_nonascii() {
    let result = diff("tests/suspicious_nonascii");
    assert_eq!(result, DiffSummary::Suspicious);
}
