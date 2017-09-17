extern crate integrity_checker;

use integrity_checker::database::{Database, DiffSummary};

#[test]
fn check() {
    let threads = 1;
    let before_path = "tests/nochanges/before";
    let after_path = "tests/nochanges/after";
    let before_db = Database::build(&before_path, false, threads).unwrap();
    let result = before_db.check(&after_path, threads).unwrap();
    assert_eq!(result, DiffSummary::NoChanges);
}
