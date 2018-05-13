extern crate integrity_checker;

extern crate flate2;

extern crate serde_json;

extern crate tempfile;

extern crate valico;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

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

fn validate(path: impl AsRef<Path>) -> Result<bool, Error> {
    let threads = 1;
    let db = Database::build(&path, false, threads)?;

    // Dump the databse to a temporary file and read it back so that
    // we can be 100% sure we're doing everything the same way as the
    // main client.
    let f = tempfile()?;
    let mut f = db.dump_json(f)?;
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
    assert!(validate("tests/nochanges/before").unwrap());
    assert!(validate("tests/nochanges/after").unwrap());
}

#[test]
fn changes_edit() {
    assert!(validate("tests/changes_edit/before").unwrap());
    assert!(validate("tests/changes_edit/after").unwrap());
}

#[test]
fn changes_new() {
    assert!(validate("tests/changes_new/before").unwrap());
    assert!(validate("tests/changes_new/after").unwrap());
}

#[test]
fn changes_edit_bin() {
    assert!(validate("tests/changes_edit_bin/before").unwrap());
    assert!(validate("tests/changes_edit_bin/after").unwrap());
}

#[test]
fn changes_new_bin() {
    assert!(validate("tests/changes_new_bin/before").unwrap());
    assert!(validate("tests/changes_new_bin/after").unwrap());
}

#[test]
fn changes_delete() {
    assert!(validate("tests/changes_delete/before").unwrap());
    assert!(validate("tests/changes_delete/after").unwrap());
}

#[test]
fn changes_delete_dir() {
    assert!(validate("tests/changes_delete_dir/before").unwrap());
    assert!(validate("tests/changes_delete_dir/after").unwrap());
}

#[test]
fn suspicious_truncate() {
    assert!(validate("tests/suspicious_truncate/before").unwrap());
    assert!(validate("tests/suspicious_truncate/after").unwrap());
}

#[test]
fn suspicious_nul() {
    assert!(validate("tests/suspicious_nul/before").unwrap());
    assert!(validate("tests/suspicious_nul/after").unwrap());
}

#[test]
fn suspicious_nonascii() {
    assert!(validate("tests/suspicious_nonascii/before").unwrap());
    assert!(validate("tests/suspicious_nonascii/after").unwrap());
}
