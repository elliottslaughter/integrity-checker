extern crate integrity_checker;

use std::path::{Path, PathBuf};

use integrity_checker::database::{Database, DiffSummary, Features};

fn diff(root_dir: impl AsRef<Path>, before_features: Features, after_features: Features) -> DiffSummary {
    let mut before_path = PathBuf::from(root_dir.as_ref());
    before_path.push("before");

    let mut after_path = PathBuf::from(root_dir.as_ref());
    after_path.push("after");

    let threads = 1;
    let before_db = Database::build(&before_path, before_features, threads, false).unwrap();
    let after_db = Database::build(&after_path, after_features, threads, false).unwrap();
    before_db.show_diff(&after_db)
}

const NONE:    Features = Features { sha2: false, blake2b: false };
const SHA2:    Features = Features { sha2:  true, blake2b: false };
const BLAKE2B: Features = Features { sha2: false, blake2b: true };
const ALL:     Features = Features { sha2:  true, blake2b: true };

const ALL_FEATURES: &[Features] = &[NONE, SHA2, BLAKE2B, ALL];

// These pairs of features share at least one hash in common (and
// therefore can detect changes even when other metrics don't change).
const VIABLE_FEATURES: &[(Features, Features)] = &[
    (   SHA2,     ALL),
    (    ALL,    SHA2),
    (BLAKE2B,     ALL),
    (    ALL, BLAKE2B),
    (    ALL,     ALL),
];

// These pairs of features don't share any common hash (and therefore
// can't detect changes except when another metric changes).
const NONVIABLE_FEATURES: &[(Features, Features)] = &[
    (   NONE,    NONE),
    (   NONE,    SHA2),
    (   SHA2,    NONE),
    (   NONE, BLAKE2B),
    (BLAKE2B,    NONE),
    (   SHA2, BLAKE2B),
    (BLAKE2B,    SHA2),
];

#[test]
fn no_changes() {
    for before_features in ALL_FEATURES {
        for after_features in ALL_FEATURES {
            let result = diff("tests/nochanges", *before_features, *after_features);
            assert_eq!(result, DiffSummary::NoChanges);
        }
    }
}

#[test]
fn changes_edit() {
    for before_features in ALL_FEATURES {
        for after_features in ALL_FEATURES {
            let result = diff("tests/changes_edit", *before_features, *after_features);
            assert_eq!(result, DiffSummary::Changes);
        }
    }
}

#[test]
fn changes_edit_no_size_change() {
    for (before_features, after_features) in VIABLE_FEATURES {
        let result = diff("tests/changes_edit_no_size_change", *before_features, *after_features);
        assert_eq!(result, DiffSummary::Changes);
    }
    for (before_features, after_features) in NONVIABLE_FEATURES {
        let result = diff("tests/changes_edit_no_size_change", *before_features, *after_features);
        assert_eq!(result, DiffSummary::NoChanges);
    }
}

#[test]
fn changes_new() {
    for before_features in ALL_FEATURES {
        for after_features in ALL_FEATURES {
            let result = diff("tests/changes_new", *before_features, *after_features);
            assert_eq!(result, DiffSummary::Changes);
        }
    }
}

#[test]
fn changes_edit_bin() {
    for (before_features, after_features) in VIABLE_FEATURES {
        let result = diff("tests/changes_edit_bin", *before_features, *after_features);
        assert_eq!(result, DiffSummary::Changes);
    }
    for (before_features, after_features) in NONVIABLE_FEATURES {
        let result = diff("tests/changes_edit_bin", *before_features, *after_features);
        assert_eq!(result, DiffSummary::NoChanges);
    }
}

#[test]
fn changes_new_bin() {
    for before_features in ALL_FEATURES {
        for after_features in ALL_FEATURES {
            let result = diff("tests/changes_new_bin", *before_features, *after_features);
            assert_eq!(result, DiffSummary::Changes);
        }
    }
}

#[test]
fn changes_delete() {
    for before_features in ALL_FEATURES {
        for after_features in ALL_FEATURES {
            let result = diff("tests/changes_delete", *before_features, *after_features);
            assert_eq!(result, DiffSummary::Changes);
        }
    }
}

#[test]
fn changes_delete_dir() {
    for before_features in ALL_FEATURES {
        for after_features in ALL_FEATURES {
            let result = diff("tests/changes_delete_dir", *before_features, *after_features);
            assert_eq!(result, DiffSummary::Changes);
        }
    }
}

#[test]
fn suspicious_truncate() {
    for before_features in ALL_FEATURES {
        for after_features in ALL_FEATURES {
            let result = diff("tests/suspicious_truncate", *before_features, *after_features);
            assert_eq!(result, DiffSummary::Suspicious);
        }
    }
}

#[test]
fn suspicious_nul() {
    for before_features in ALL_FEATURES {
        for after_features in ALL_FEATURES {
            let result = diff("tests/suspicious_nul", *before_features, *after_features);
            assert_eq!(result, DiffSummary::Suspicious);
        }
    }
}

#[test]
fn suspicious_nonascii() {
    for before_features in ALL_FEATURES {
        for after_features in ALL_FEATURES {
            let result = diff("tests/suspicious_nonascii", *before_features, *after_features);
            assert_eq!(result, DiffSummary::Suspicious);
        }
    }
}
