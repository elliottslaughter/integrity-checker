extern crate clap;

extern crate serde_cbor;
extern crate serde_json;

extern crate integrity_checker;

use std::ffi::OsString;
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

    {
        let mut json_path = PathBuf::from(&db_path);
        json_path.set_extension("json");
        database.dump_json(json_path).unwrap();
    }

    {
        let mut cbor_path = PathBuf::from(&db_path);
        cbor_path.set_extension("cbor");
        database.dump_cbor(cbor_path).unwrap();
    }

    let json = serde_json::to_string(&database).unwrap();
    println!("JSON bytes: {}", json.len());
    let cbor = serde_cbor::to_vec(&database).unwrap();
    println!("CBOR bytes: {}", cbor.len());
}
