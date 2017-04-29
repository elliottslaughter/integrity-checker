extern crate clap;
extern crate generic_array;
extern crate ignore;

extern crate digest;
extern crate sha2;

extern crate integrity_checker;

use std::default::Default;
use std::ffi::OsString;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use generic_array::GenericArray;
use ignore::WalkBuilder;
use digest::Digest;

use integrity_checker::error;

#[derive(Debug, Clone, PartialEq)]
struct HashSum<T: Digest>(GenericArray<u8, <T as Digest>::OutputSize>);

fn parse_args() -> OsString {
    let matches = clap::App::new("Integrity Checker")
        .arg(clap::Arg::with_name("path")
             .help("Path to file or directory to check integrity of")
             .required(true)
             .index(1))
        .get_matches();
    matches.value_of_os("path").unwrap().to_owned()
}

fn compute_hash<D, P>(path: P) -> Result<HashSum<D>, error::Error>
where
    D: Digest + Default,
    P: AsRef<Path>,
{
    let mut f = File::open(path)?;

    let mut hasher = D::default();

    let mut buffer = [0; 4096];

    loop {
        let n = f.read(&mut buffer[..])?;
        if n == 0 { break }
        hasher.input(&buffer[0..n]);
    }
    Ok(HashSum(hasher.result()))
}

fn walk_directory<P: AsRef<Path>>(path: P) -> Result<(), error::Error> {
    for entry in WalkBuilder::new(path).build() {
        let entry = entry?;
        if entry.file_type().map_or(false, |t| t.is_file()) {
            let hash: HashSum<sha2::Sha256> = compute_hash(entry.path())?;
            let result = hash.0.map(|b| format!("{:02x}", b)).join("");
            println!("{}  {}", result, entry.path().display());
        }
    }
    Ok(())
}

fn main() {
    let path = parse_args();
    walk_directory(&path).unwrap();
}
