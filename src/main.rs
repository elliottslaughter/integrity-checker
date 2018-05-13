extern crate clap;

extern crate serde_json;

extern crate integrity_checker;

use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

use integrity_checker::database::{Database, DiffSummary};
use integrity_checker::error;

enum Action {
    Build { db_path: OsString, dir_path: OsString, force: bool, threads: usize },
    Check { db_path: OsString, dir_path: OsString, threads: usize },
    Diff { old_path: OsString, new_path: OsString },
    SelfCheck { db_path: OsString },
}

#[derive(Debug)]
enum ActionSummary {
    Built,
    Diff(DiffSummary),
}

fn validate_usize(s: String) -> Result<(), String> {
    s.parse::<usize>().map(|_| ()).map_err(|e| e.to_string())
}

fn parse_args() -> Action {
    let matches = clap::App::new("Integrity Checker")
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
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
                    .arg(clap::Arg::with_name("force")
                         .help("Overwrite existing file")
                         .short("f").long("force"))
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
        .subcommand(clap::SubCommand::with_name("selfcheck")
                    .about("Check the internal consistency of an integrity database")
                    .arg(clap::Arg::with_name("database")
                         .help("Path of integrity database to read")
                         .required(true)
                         .index(1)))
        .after_help("RETURN CODE: \
                    \n    0       Success \
                    \n    1       Changes \
                    \n    2       Suspicious changes \
                    \n   -1       Error")
        .get_matches();
    match matches.subcommand() {
        ("build", Some(submatches)) => Action::Build {
            db_path: submatches.value_of_os("database").unwrap().to_owned(),
            dir_path: submatches.value_of_os("path").unwrap().to_owned(),
            force: submatches.is_present("force"),
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
        ("selfcheck", Some(submatches)) => Action::SelfCheck {
            db_path: submatches.value_of_os("database").unwrap().to_owned(),
        },
        _ => unreachable!(),
    }
}

fn driver() -> Result<ActionSummary, error::Error> {
    let action = parse_args();
    match action {
        Action::Build { db_path, dir_path, force, threads } => {
            let mut json_path = PathBuf::from(&db_path);
            json_path.set_extension("json.gz");
            // Truncate only when force is set
            let f = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .create_new(!force)
                .open(json_path)?;

            let database = Database::build(&dir_path, true, threads)?;
            database.dump_json(f)?;

            Ok(ActionSummary::Built)
        }
        Action::Check { db_path, dir_path, threads } => {
            let mut json_path = PathBuf::from(&db_path);
            json_path.set_extension("json.gz");
            let f = File::open(json_path)?;
            let database = Database::load_json(f)?;
            Ok(ActionSummary::Diff(database.check(&dir_path, threads)?))
        }
        Action::Diff { old_path, new_path } => {
            let mut json_old_path = PathBuf::from(&old_path);
            json_old_path.set_extension("json.gz");
            let mut json_new_path = PathBuf::from(&new_path);
            json_new_path.set_extension("json.gz");
            let f_old = File::open(json_old_path)?;
            let f_new = File::open(json_new_path)?;
            let old = Database::load_json(f_old)?;
            let new = Database::load_json(f_new)?;
            Ok(ActionSummary::Diff(old.show_diff(&new)))
        }
        Action::SelfCheck { db_path } => {
            let mut json_path = PathBuf::from(&db_path);
            json_path.set_extension("json.gz");
            let f = File::open(json_path)?;
            Database::load_json(f)?;
            Ok(ActionSummary::Diff(DiffSummary::NoChanges))
        }
    }
}

fn main() {
    ::std::process::exit(match driver() {
       Ok(action_summary) => match action_summary {
           ActionSummary::Built => 0,
           ActionSummary::Diff(DiffSummary::NoChanges) => 0,
           ActionSummary::Diff(DiffSummary::Changes) => 1,
           ActionSummary::Diff(DiffSummary::Suspicious) => 2,
       },
       Err(err) => {
           writeln!(io::stderr(), "error: {:?}", err).unwrap();
           -1
       },
    });
}
