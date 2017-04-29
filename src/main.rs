extern crate clap;
extern crate ignore;
extern crate sha2;

use std::ffi::OsString;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use sha2::Digest;
use ignore::WalkBuilder;

#[derive(Debug)]
enum Error {
    Io(std::io::Error),
    Ignore(ignore::Error),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<ignore::Error> for Error {
    fn from(err: ignore::Error) -> Error {
        Error::Ignore(err)
    }
}

fn parse_args() -> OsString {
    let matches = clap::App::new("Integrity Checker")
        .arg(clap::Arg::with_name("path")
             .help("Path to file or directory to check integrity of")
             .required(true)
             .index(1))
        .get_matches();
    matches.value_of_os("path").unwrap().to_owned()
}

fn compute_hash<P: AsRef<Path>>(path: P) -> Result<String, std::io::Error> {
    let mut f = File::open(path)?;

    let mut hasher = sha2::Sha256::default();

    let mut buffer = [0; 4096];

    loop {
        let n = f.read(&mut buffer[..])?;
        if n == 0 { break }
        hasher.input(&buffer[0..n]);
    }
    Ok(hasher.result().map(|b| format!("{:02x}", b)).join(""))
}

// FIXME: I'm throwing away the extra info in walkdir::Error here. But
// walkdir::Error doesn't provide a From or any way to construct one.
fn walk_directory<P: AsRef<Path>>(path: P) -> Result<(), Error> {
    for entry in WalkBuilder::new(path).build() {
        let entry = entry?;
        if entry.file_type().map_or(false, |t| t.is_file()) {
            let hash = compute_hash(entry.path())?;
            println!("{}  {}", hash, entry.path().display());
        }
    }
    Ok(())
}

fn main() {
    let path = parse_args();
    walk_directory(&path).unwrap();
}
