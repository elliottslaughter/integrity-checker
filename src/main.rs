extern crate clap;

extern crate serde_cbor;
extern crate serde_json;

extern crate integrity_checker;

use std::ffi::OsString;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use integrity_checker::database::Database;

fn parse_args() -> (OsString, OsString) {
    let matches = clap::App::new("Integrity Checker")
        .arg(clap::Arg::with_name("database")
             .help("Path to integrity database")
             .required(true)
             .index(1))
        .arg(clap::Arg::with_name("path")
             .help("Path to file or directory to check integrity of")
             .required(true)
             .index(2))
        .get_matches();
    (matches.value_of_os("database").unwrap().to_owned(),
     matches.value_of_os("path").unwrap().to_owned())
}

fn main() {
    let (db_path, dir_path) = parse_args();
    let database = Database::build(&dir_path).unwrap();
    let json = serde_json::to_string(&database).unwrap();

    {
        let mut json_path = PathBuf::from(&db_path);
        json_path.set_extension("json");
        let mut json_f = File::create(json_path).unwrap();
        write!(json_f, "{}", json).unwrap();
    }

    let cbor = serde_cbor::to_vec(&database).unwrap();

    {
        let mut cbor_path = PathBuf::from(&db_path);
        cbor_path.set_extension("cbor");
        let mut cbor_f = File::create(cbor_path).unwrap();
        cbor_f.write_all(cbor.as_slice()).unwrap();
    }

    println!("JSON bytes: {}", json.len());
    println!("CBOR bytes: {}", cbor.len());
}
