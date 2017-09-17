extern crate integrity_checker;

use std::path::Path;

use integrity_checker::database::{Database, DiffSummary};

fn check<P>(before_path: P, after_path: P) -> DiffSummary
where
    P: AsRef<Path>,
{
    let threads = 1;
    let before_db = Database::build(&before_path, false, threads).unwrap();
    before_db.check(&after_path, threads).unwrap()
}

#[test]
fn no_changes() {
    let result = check("tests/nochanges/before", "tests/nochanges/after");
    assert_eq!(result, DiffSummary::NoChanges);
}

#[test]
fn changes_edit() {
    let result = check("tests/changes_edit/before", "tests/changes_edit/after");
    assert_eq!(result, DiffSummary::Changes);
}

#[test]
fn changes_new() {
    let result = check("tests/changes_new/before", "tests/changes_new/after");
    assert_eq!(result, DiffSummary::Changes);
}

#[test]
fn changes_edit_bin() {
    let result = check("tests/changes_edit_bin/before",
                      "tests/changes_edit_bin/after");
    assert_eq!(result, DiffSummary::Changes);
}

#[test]
fn changes_new_bin() {
    let result = check("tests/changes_new_bin/before",
                      "tests/changes_new_bin/after");
    assert_eq!(result, DiffSummary::Changes);
}

#[test]
fn changes_delete() {
    let result = check("tests/changes_delete/before",
                      "tests/changes_delete/after");
    assert_eq!(result, DiffSummary::Changes);
}
