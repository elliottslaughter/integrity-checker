extern crate clap;

extern crate serde_cbor;
extern crate serde_json;
extern crate rmp_serde;

extern crate integrity_checker;

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::PathBuf;

use integrity_checker::database::{Database, DiffCode};
use integrity_checker::error;

enum Action {
    Build { db_path: OsString, dir_path: OsString, threads: usize },
    Check { db_path: OsString, dir_path: OsString, threads: usize },
    Diff { old_path: OsString, new_path: OsString },
}

#[derive(Debug)]
enum ReturnCode {
    Built,
    Succeeded(DiffCode),
}

fn validate_usize(s: String) -> Result<(), String> {
    s.parse::<usize>().map(|_| ()).map_err(|e| e.to_string())
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
                         .index(2))
                    .arg(clap::Arg::with_name("threads")
                         .help("Number of threads to use")
                         .short("j").long("threads")
                         .takes_value(true)
                         .validator(validate_usize)))
        .subcommand(clap::SubCommand::with_name("check")
                    .about("Check an integrity database against a directory")
                    .arg(clap::Arg::with_name("database")
                         .help("Path of integrity database to read")
                         .required(true)
                         .index(1))
                    .arg(clap::Arg::with_name("path")
                         .help("Path of file or directory to scan")
                         .required(true)
                         .index(2))
                    .arg(clap::Arg::with_name("threads")
                         .help("Number of threads to use")
                         .short("j").long("threads")
                         .takes_value(true)
                         .validator(validate_usize)))
        .subcommand(clap::SubCommand::with_name("diff")
                    .about("Compare two integrity databases")
                    .arg(clap::Arg::with_name("old")
                         .help("Path of old integrity database")
                         .required(true)
                         .index(1))
                    .arg(clap::Arg::with_name("new")
                         .help("Path of new integrity database")
                         .required(true)
                         .index(2)))
        .get_matches();
    match matches.subcommand() {
        ("build", Some(submatches)) => Action::Build {
            db_path: submatches.value_of_os("database").unwrap().to_owned(),
            dir_path: submatches.value_of_os("path").unwrap().to_owned(),
            threads: match submatches.value_of("threads") {
                None => 1, // FIXME: Pick a reasonable number of threads
                Some(threads) => threads.parse().unwrap(),
            },
        },
        ("check", Some(submatches)) => Action::Check {
            db_path: submatches.value_of_os("database").unwrap().to_owned(),
            dir_path: submatches.value_of_os("path").unwrap().to_owned(),
            threads: match submatches.value_of("threads") {
                None => 1, // FIXME: Pick a reasonable number of threads
                Some(threads) => threads.parse().unwrap(),
            },
        },
        ("diff", Some(submatches)) => Action::Diff {
            old_path: submatches.value_of_os("old").unwrap().to_owned(),
            new_path: submatches.value_of_os("new").unwrap().to_owned(),
        },
        _ => unreachable!(),
    }
}

fn run_app() -> Result<ReturnCode, error::Error> {
    let action = parse_args();
    match action {
        Action::Build { db_path, dir_path, threads } => {
            let database = Database::build(&dir_path, true, threads)?;

            {
                let mut json_path = PathBuf::from(&db_path);
                json_path.set_extension("json");
                database.dump_json(json_path)?;
            }

            {
                let mut cbor_path = PathBuf::from(&db_path);
                cbor_path.set_extension("cbor");
                database.dump_cbor(cbor_path)?;
            }

            {
                let mut msgpack_path = PathBuf::from(&db_path);
                msgpack_path.set_extension("msgpack");
                database.dump_msgpack(msgpack_path)?;
            }

            let json = serde_json::to_string(&database)?;
            println!("JSON bytes: {}", json.len());
            let cbor = serde_cbor::to_vec(&database)?;
            println!("CBOR bytes: {}", cbor.len());
            let msgpack = rmp_serde::to_vec(&database)?;
            println!("MsgPack bytes: {}", msgpack.len());
            Ok(ReturnCode::Built)
        }
        Action::Check { db_path, dir_path, threads } => {
            let mut cbor_path = PathBuf::from(&db_path);
            cbor_path.set_extension("cbor");
            let database = Database::load_cbor(&cbor_path)?;
            Ok(ReturnCode::Succeeded(database.check(&dir_path, threads)?))
        }
        Action::Diff { old_path, new_path } => {
            let mut cbor_old_path = PathBuf::from(&old_path);
            cbor_old_path.set_extension("cbor");
            let mut cbor_new_path = PathBuf::from(&new_path);
            cbor_new_path.set_extension("cbor");
            let old = Database::load_cbor(&cbor_old_path)?;
            let new = Database::load_cbor(&cbor_new_path)?;
            Ok(ReturnCode::Succeeded(old.show_diff(&new)))
        }
    }
}

fn main() {
    ::std::process::exit(match run_app() {
       Ok(return_code) => match return_code {
           ReturnCode::Succeeded(diff_code) => diff_code as i32,
           _ => 0,
       },
       Err(err) => {
           writeln!(io::stderr(), "error: {:?}", err).unwrap();
           -1
       },
    });
}
