extern crate clap;
extern crate generic_array;
extern crate ignore;

extern crate digest;
extern crate sha2;
extern crate sha3;

extern crate integrity_checker;

use std::collections::BTreeMap;
use std::default::Default;
use std::ffi::OsString;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use generic_array::{ArrayLength, GenericArray};
use ignore::WalkBuilder;
use digest::Digest;

use integrity_checker::error;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Database(BTreeMap<OsString, Entry>);

#[derive(Debug, Clone, PartialEq, Eq)]
enum Entry {
    File(HashSums),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HashSums {
    sha2: HashSum<<sha2::Sha256 as Digest>::OutputSize>,
    sha3: HashSum<<sha3::Sha3_256 as Digest>::OutputSize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HashSum<T: ArrayLength<u8>>(GenericArray<u8, T>);

fn parse_args() -> OsString {
    let matches = clap::App::new("Integrity Checker")
        .arg(clap::Arg::with_name("path")
             .help("Path to file or directory to check integrity of")
             .required(true)
             .index(1))
        .get_matches();
    matches.value_of_os("path").unwrap().to_owned()
}

#[derive(Clone, Default)]
struct Hashers(sha2::Sha256, sha3::Sha3_256);

impl Hashers {
    fn input(&mut self, input: &[u8]) {
        self.0.input(input);
        self.1.input(input);
    }
    fn result(self) -> HashSums {
        HashSums {
            sha2: HashSum(self.0.result()),
            sha3: HashSum(self.1.result()),
        }
    }
}

fn compute_hashes<P>(path: P) -> Result<HashSums, error::Error>
where
    P: AsRef<Path>
{
    let mut f = File::open(path)?;

    let mut hashers = Hashers::default();

    let mut buffer = [0; 4096];
    loop {
        let n = f.read(&mut buffer[..])?;
        if n == 0 { break }
        hashers.input(&buffer[0..n]);
    }
    Ok(hashers.result())
}

fn build_database<P: AsRef<Path>>(path: P) -> Result<Database, error::Error> {
    let mut database = Database::default();
    for entry in WalkBuilder::new(path).build() {
        let entry = entry?;
        if entry.file_type().map_or(false, |t| t.is_file()) {
            let path = entry.path().as_os_str().to_owned();
            let hashes = compute_hashes(entry.path())?;
            let result = Entry::File(hashes);
            database.0.insert(path, result);
        }
    }
    Ok(database)
}

impl std::fmt::Display for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (path, entry) in self.0.iter() {
            match entry {
                &Entry::File(ref hashes) => writeln!(
                    f, "{} {}",
                    hashes.sha2.0.map(|b| format!("{:02x}", b)).join(""),
                    Path::new(path).display())?
            }
        }
        Ok(())
    }
}

fn main() {
    let path = parse_args();
    let database = build_database(&path).unwrap();
    println!("{}", database);
}
