extern crate clap;
extern crate walkdir;
extern crate sha2;

use std::ffi::OsString;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use sha2::Digest;
use walkdir::WalkDir;

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

    let mut hasher = sha2::Sha256::new();

    let mut buffer = [0; 4096];

    loop {
        let n = f.read(&mut buffer[..])?;
        if n == 0 { break }
        hasher.input(&buffer[0..n]);
    }
    Ok(hasher.result().map(|b| format!("{:x}", b)).join(""))
}

// FIXME: I'm throwing away the extra info in walkdir::Error here. But
// walkdir::Error doesn't provide a From or any way to construct one.
fn walk_directory<P: AsRef<Path>>(path: P) -> Result<(), std::io::Error> {
    for entry in WalkDir::new(path) {
        let entry = entry?;
        if entry.file_type().is_file() {
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
