extern crate integrity_checker;

extern crate flate2;

extern crate serde_json;

extern crate tempfile;

extern crate valico;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use integrity_checker::database::{Database, Features};
use integrity_checker::error::Error;

use flate2::read::GzDecoder;

use serde_json::Value;

use tempfile::tempfile;

use valico::json_schema;

fn validate_schema(data: &[u8], schema_path: impl AsRef<Path>) -> Result<bool, Error> {
    let instance: Value = serde_json::from_slice(data)?;

    let f = File::open(schema_path)?;
    let schema: Value = serde_json::from_reader(f)?;

    let mut scope = json_schema::Scope::new();
    let schema = scope.compile_and_return(schema, true).unwrap();
    Ok(schema.validate(&instance).is_valid())
}

fn validate(path: impl AsRef<Path>, features: Features) -> Result<bool, Error> {
    let threads = 1;
    let db = Database::build(&path, features, threads, false)?;

    // Dump the databse to a temporary file and read it back so that
    // we can be 100% sure we're doing everything the same way as the
    // main client.
    let f = tempfile()?;
    let mut f = db.dump_json(f, features)?;
    f.seek(SeekFrom::Start(0))?;
    let mut d = GzDecoder::new(f);
    let mut bytes = Vec::new();
    d.read_to_end(&mut bytes)?;
    let bytes = bytes;

    // Now make sure the format validates.
    const SEP: u8 = 0x0a;
    let index = match bytes.iter().position(|&x| x == SEP) {
        Some(x) => x,
        None => return Err(Error::ParseError),
    };

    Ok(validate_schema(&bytes[..index], "schema/checksum.json")? &&
       validate_schema(&bytes[index+1..], "schema/database.json")?)
}

const NONE:    Features = Features { sha2: false, blake2b: false };
const SHA2:    Features = Features { sha2:  true, blake2b: false };
const BLAKE2B: Features = Features { sha2: false, blake2b: true };
const ALL:     Features = Features { sha2:  true, blake2b: true };

const ALL_FEATURES: &[Features] = &[NONE, SHA2, BLAKE2B, ALL];

#[test]
fn no_changes() {
    for features in ALL_FEATURES {
        assert!(validate("tests/nochanges/before", *features).unwrap());
        assert!(validate("tests/nochanges/after", *features).unwrap());
    }
}

#[test]
fn changes_edit() {
    for features in ALL_FEATURES {
        assert!(validate("tests/changes_edit/before", *features).unwrap());
        assert!(validate("tests/changes_edit/after", *features).unwrap());
    }
}

#[test]
fn changes_edit_no_size_change() {
    for features in ALL_FEATURES {
        assert!(validate("tests/changes_edit_no_size_change/before", *features).unwrap());
        assert!(validate("tests/changes_edit_no_size_change/after", *features).unwrap());
    }
}

#[test]
fn changes_new() {
    for features in ALL_FEATURES {
        assert!(validate("tests/changes_new/before", *features).unwrap());
        assert!(validate("tests/changes_new/after", *features).unwrap());
    }
}

#[test]
fn changes_edit_bin() {
    for features in ALL_FEATURES {
        assert!(validate("tests/changes_edit_bin/before", *features).unwrap());
        assert!(validate("tests/changes_edit_bin/after", *features).unwrap());
    }
}

#[test]
fn changes_new_bin() {
    for features in ALL_FEATURES {
        assert!(validate("tests/changes_new_bin/before", *features).unwrap());
        assert!(validate("tests/changes_new_bin/after", *features).unwrap());
    }
}

#[test]
fn changes_delete() {
    for features in ALL_FEATURES {
        assert!(validate("tests/changes_delete/before", *features).unwrap());
        assert!(validate("tests/changes_delete/after", *features).unwrap());
    }
}

#[test]
fn changes_delete_dir() {
    for features in ALL_FEATURES {
        assert!(validate("tests/changes_delete_dir/before", *features).unwrap());
        assert!(validate("tests/changes_delete_dir/after", *features).unwrap());
    }
}

#[test]
fn suspicious_truncate() {
    for features in ALL_FEATURES {
        assert!(validate("tests/suspicious_truncate/before", *features).unwrap());
        assert!(validate("tests/suspicious_truncate/after", *features).unwrap());
    }
}

#[test]
fn suspicious_nul() {
    for features in ALL_FEATURES {
        assert!(validate("tests/suspicious_nul/before", *features).unwrap());
        assert!(validate("tests/suspicious_nul/after", *features).unwrap());
    }
}

#[test]
fn suspicious_nonascii() {
    for features in ALL_FEATURES {
        assert!(validate("tests/suspicious_nonascii/before", *features).unwrap());
        assert!(validate("tests/suspicious_nonascii/after", *features).unwrap());
    }
}
