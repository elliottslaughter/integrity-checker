extern crate integrity_checker;

extern crate flate2;

extern crate serde_json;

extern crate tempfile;

extern crate valico;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use integrity_checker::database::Database;
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

fn validate(root_dir: impl AsRef<Path>) -> Result<bool, Error> {
    let mut before_path = PathBuf::from(root_dir.as_ref());
    before_path.push("before");

    let mut after_path = PathBuf::from(root_dir.as_ref());
    after_path.push("after");

    let threads = 1;
    let before_db = Database::build(&before_path, false, threads)?;

    // Dump the databse to a temporary file and read it back so that
    // we can be 100% we're doing everything the same was as the main
    // client.
    let f = tempfile()?;
    let mut f = before_db.dump_json(f)?;
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

#[test]
fn no_changes() {
    assert!(validate("tests/nochanges").unwrap());
}

#[test]
fn changes_edit() {
    assert!(validate("tests/changes_edit").unwrap());
}

#[test]
fn changes_new() {
    assert!(validate("tests/changes_new").unwrap());
}

#[test]
fn changes_edit_bin() {
    assert!(validate("tests/changes_edit_bin").unwrap());
}

#[test]
fn changes_new_bin() {
    assert!(validate("tests/changes_new_bin").unwrap());
}

#[test]
fn changes_delete() {
    assert!(validate("tests/changes_delete").unwrap());
}

#[test]
fn suspicious_truncate() {
    assert!(validate("tests/suspicious_truncate").unwrap());
}

#[test]
fn suspicious_nul() {
    assert!(validate("tests/suspicious_nul").unwrap());
}

#[test]
fn suspicious_nonascii() {
    assert!(validate("tests/suspicious_nonascii").unwrap());
}
