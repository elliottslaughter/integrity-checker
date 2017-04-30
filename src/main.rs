extern crate clap;

extern crate serde_cbor;
extern crate serde_json;

extern crate integrity_checker;

use std::ffi::OsString;
use std::path::PathBuf;

use integrity_checker::database::Database;

enum Action {
    Build { db_path: OsString, dir_path: OsString },
}

fn parse_args() -> Action {
    let matches = clap::App::new("Integrity Checker")
        .subcommand(clap::SubCommand::with_name("build")
                    .about("Creates an integrity database from a directory")
                    .arg(clap::Arg::with_name("database")
                         .help("Path of integrity database to create")
                         .required(true)
                         .index(1))
                    .arg(clap::Arg::with_name("path")
                         .help("Path of file or directory to scan")
                         .required(true)
                         .index(2)))
        .get_matches();
    match matches.subcommand() {
        ("build", Some(submatches)) => Action::Build {
            db_path: submatches.value_of_os("database").unwrap().to_owned(),
            dir_path: submatches.value_of_os("path").unwrap().to_owned(),
        },
        _ => unreachable!(),
    }
}

fn main() {
    let action = parse_args();
    match action {
        Action::Build { db_path, dir_path } => {
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
    }
}
