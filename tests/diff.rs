extern crate integrity_checker;

use std::path::Path;

use integrity_checker::database::{Database, DiffSummary};

fn diff<P>(before_path: P, after_path: P) -> DiffSummary
where
    P: AsRef<Path>,
{
    let threads = 1;
    let before_db = Database::build(&before_path, false, threads).unwrap();
    let after_db = Database::build(&after_path, false, threads).unwrap();
    before_db.show_diff(&after_db)
}

#[test]
fn no_changes() {
    let result = diff("tests/nochanges/before", "tests/nochanges/after");
    assert_eq!(result, DiffSummary::NoChanges);
}

#[test]
fn changes_edit() {
    let result = diff("tests/changes_edit/before", "tests/changes_edit/after");
    assert_eq!(result, DiffSummary::Changes);
}

#[test]
fn changes_new() {
    let result = diff("tests/changes_new/before", "tests/changes_new/after");
    assert_eq!(result, DiffSummary::Changes);
}

#[test]
fn changes_edit_bin() {
    let result = diff("tests/changes_edit_bin/before",
                      "tests/changes_edit_bin/after");
    assert_eq!(result, DiffSummary::Changes);
}

#[test]
fn changes_new_bin() {
    let result = diff("tests/changes_new_bin/before",
                      "tests/changes_new_bin/after");
    assert_eq!(result, DiffSummary::Changes);
}
