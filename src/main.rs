#[macro_use]
extern crate clap;

use std::ffi::OsString;
use std::fs::{File, OpenOptions};

use integrity_checker::database::{Database, DiffSummary, Features};
use integrity_checker::error;

enum Action {
    Build {
        db_path: OsString,
        dir_path: OsString,
        features: Features,
        threads: usize,
        force: bool,
    },
    Check {
        db_path: OsString,
        dir_path: OsString,
        features: Features,
        threads: usize,
    },
    Diff {
        old_path: OsString,
        new_path: OsString,
    },
    SelfCheck {
        db_path: OsString,
    },
}

#[derive(Debug)]
enum ActionSummary {
    Built,
    Diff(DiffSummary),
}

fn validate_usize(s: &str) -> Result<(), String> {
    s.parse::<usize>().map(|_| ()).map_err(|e| e.to_string())
}

trait DefaultFlags {
    fn add_default_flags(self) -> Self;
}

impl<'a> DefaultFlags for clap::App<'a> {
    fn add_default_flags(self) -> Self {
        self.arg(
            clap::Arg::with_name("threads")
                .help("Number of threads to use")
                .short('j')
                .long("threads")
                .takes_value(true)
                .validator(validate_usize),
        )
        .arg(
            clap::Arg::with_name("sha2")
                .help("Enable use of SHA2-256/512 algorithm")
                .long("sha2")
                .overrides_with("no-sha2"),
        )
        .arg(
            clap::Arg::with_name("no-sha2")
                .help("Disable use of SHA2-256/512 algorithm")
                .long("no-sha2")
                .overrides_with("sha2"),
        )
        .arg(
            clap::Arg::with_name("blake2")
                .help("Enable use of BLAKE2b algorithm")
                .long("blake2")
                .overrides_with("no-blake2"),
        )
        .arg(
            clap::Arg::with_name("no-blake2")
                .help("Disable use of BLAKE2b algorithm")
                .long("no-blake2")
                .overrides_with("blake2"),
        )
    }
}

fn parse_features(matches: &clap::ArgMatches) -> Features {
    let defaults = Features::default();

    let sha2 = if matches.is_present("sha2") {
        true
    } else if matches.is_present("no-sha2") {
        false
    } else {
        defaults.sha2
    };

    let blake2b = if matches.is_present("blake2") {
        true
    } else if matches.is_present("no-blake2") {
        false
    } else {
        defaults.blake2b
    };

    Features { sha2, blake2b }
}

fn parse_threads(matches: &clap::ArgMatches) -> usize {
    match matches.value_of("threads") {
        None => 1, // FIXME: Pick a reasonable number of threads
        Some(threads) => threads.parse().unwrap(),
    }
}

fn parse_args() -> Action {
    let matches = clap::App::new("Integrity Checker")
        .version(crate_version!())
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            clap::SubCommand::with_name("build")
                .about("Creates an integrity database from a directory")
                .arg(
                    clap::Arg::with_name("database")
                        .help("Path of integrity database to create")
                        .required(true)
                        .index(1),
                )
                .arg(
                    clap::Arg::with_name("path")
                        .help("Path of file or directory to scan")
                        .required(true)
                        .index(2),
                )
                .arg(
                    clap::Arg::with_name("force")
                        .help("Overwrite existing file")
                        .short('f')
                        .long("force"),
                )
                .add_default_flags(),
        )
        .subcommand(
            clap::SubCommand::with_name("check")
                .about("Check an integrity database against a directory")
                .arg(
                    clap::Arg::with_name("database")
                        .help("Path of integrity database to read")
                        .required(true)
                        .index(1),
                )
                .arg(
                    clap::Arg::with_name("path")
                        .help("Path of file or directory to scan")
                        .required(true)
                        .index(2),
                )
                .add_default_flags(),
        )
        .subcommand(
            clap::SubCommand::with_name("diff")
                .about("Compare two integrity databases")
                .arg(
                    clap::Arg::with_name("old")
                        .help("Path of old integrity database")
                        .required(true)
                        .index(1),
                )
                .arg(
                    clap::Arg::with_name("new")
                        .help("Path of new integrity database")
                        .required(true)
                        .index(2),
                ),
        )
        .subcommand(
            clap::SubCommand::with_name("selfcheck")
                .about("Check the internal consistency of an integrity database")
                .arg(
                    clap::Arg::with_name("database")
                        .help("Path of integrity database to read")
                        .required(true)
                        .index(1),
                ),
        )
        .after_help(
            "RETURN CODE: \
                    \n    0       Success \
                    \n    1       Changes \
                    \n    2       Suspicious changes \
                    \n   -1       Error",
        )
        .get_matches();
    match matches.subcommand() {
        Some(("build", submatches)) => Action::Build {
            db_path: submatches.value_of_os("database").unwrap().to_owned(),
            dir_path: submatches.value_of_os("path").unwrap().to_owned(),
            features: parse_features(submatches),
            threads: parse_threads(submatches),
            force: submatches.is_present("force"),
        },
        Some(("check", submatches)) => Action::Check {
            db_path: submatches.value_of_os("database").unwrap().to_owned(),
            dir_path: submatches.value_of_os("path").unwrap().to_owned(),
            features: parse_features(submatches),
            threads: parse_threads(submatches),
        },
        Some(("diff", submatches)) => Action::Diff {
            old_path: submatches.value_of_os("old").unwrap().to_owned(),
            new_path: submatches.value_of_os("new").unwrap().to_owned(),
        },
        Some(("selfcheck", submatches)) => Action::SelfCheck {
            db_path: submatches.value_of_os("database").unwrap().to_owned(),
        },
        _ => unreachable!(),
    }
}

fn driver() -> Result<ActionSummary, error::Error> {
    let action = parse_args();
    match action {
        Action::Build {
            db_path,
            dir_path,
            features,
            threads,
            force,
        } => {
            // Truncate only when force is set
            let f = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .create_new(!force)
                .open(db_path)?;

            let database = Database::build(dir_path, features, threads, true)?;
            database.dump_json(f, features)?;

            Ok(ActionSummary::Built)
        }
        Action::Check {
            db_path,
            dir_path,
            features,
            threads,
        } => {
            let f = File::open(db_path)?;
            let database = Database::load_json(f)?;
            Ok(ActionSummary::Diff(
                database.check(dir_path, features, threads)?,
            ))
        }
        Action::Diff { old_path, new_path } => {
            let f_old = File::open(old_path)?;
            let f_new = File::open(new_path)?;
            let old = Database::load_json(f_old)?;
            let new = Database::load_json(f_new)?;
            Ok(ActionSummary::Diff(old.show_diff(&new)))
        }
        Action::SelfCheck { db_path } => {
            let f = File::open(db_path)?;
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
            eprintln!("error: {:?}", err);
            -1
        }
    });
}
