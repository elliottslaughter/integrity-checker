use std::collections::BTreeMap;
use std::default::Default;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use digest::Digest;
use ignore::WalkBuilder;
use serde_bytes;
use serde_cbor;
use serde_json;

use sha2;
use sha3;

use error;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Database(Entry);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Entry {
    Directory(BTreeMap<PathBuf, Entry>),
    File(Metrics),
}

impl Default for Entry {
    fn default() -> Entry {
        Entry::Directory(BTreeMap::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metrics {
    sha2: HashSum,
    sha3: HashSum,
    size: u64,      // File size
    nul: bool,      // Does the file contain a NUL byte?
    nonascii: bool, // Does the file contain non-ASCII bytes?
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashSum(#[serde(with = "serde_bytes")] Vec<u8>);

#[derive(Default)]
struct EngineSize(u64);
impl EngineSize {
    fn input(&mut self, input: &[u8]) {
        self.0 += input.len() as u64;
    }
    fn result(self) -> u64 {
        self.0
    }
}

#[derive(Default)]
struct EngineNul(bool);
impl EngineNul {
    fn input(&mut self, input: &[u8]) {
        self.0 = self.0 || input.iter().any(|x| *x == 0);
    }
    fn result(self) -> bool {
        self.0
    }
}

#[derive(Default)]
struct EngineNonascii(bool);
impl EngineNonascii {
    fn input(&mut self, input: &[u8]) {
        self.0 = self.0 || input.iter().any(|x| x & 0x80 != 0);
    }
    fn result(self) -> bool {
        self.0
    }
}

#[derive(Default)]
struct Engines {
    sha2: sha2::Sha256,
    sha3: sha3::Sha3_256,
    size: EngineSize,
    nul: EngineNul,
    nonascii: EngineNonascii,
}

impl Engines {
    fn input(&mut self, input: &[u8]) {
        self.sha2.input(input);
        self.sha3.input(input);
        self.size.input(input);
        self.nul.input(input);
        self.nonascii.input(input);
    }
    fn result(self) -> Metrics {
        Metrics {
            sha2: HashSum(Vec::from(self.sha2.result().as_slice())),
            sha3: HashSum(Vec::from(self.sha3.result().as_slice())),
            size: self.size.result(),
            nul: self.nul.result(),
            nonascii: self.nonascii.result(),
        }
    }
}

fn compute_metrics<P>(path: P) -> Result<Metrics, error::Error>
where
    P: AsRef<Path>
{
    let mut f = File::open(path)?;

    let mut engines = Engines::default();

    let mut buffer = [0; 4096];
    loop {
        let n = f.read(&mut buffer[..])?;
        if n == 0 { break }
        engines.input(&buffer[0..n]);
    }
    Ok(engines.result())
}

trait BTreeMapExt<K, V> where K: Ord, V: Default {
    fn get_mut_default(&mut self, key: K) -> &mut V;
}

impl<K, V> BTreeMapExt<K, V> for BTreeMap<K, V>
where
    K: Ord + Clone,
    V: Default,
{
    fn get_mut_default(&mut self, key: K) -> &mut V {
        if self.contains_key(&key) {
            match self.get_mut(&key) {
                Some(value) => value,
                None => unreachable!(), // Already checked that key exists
            }
        } else {
            {
                // FIXME: Would prefer to avoid the clone
                match self.insert(key.clone(), V::default()) {
                    Some(_) => unreachable!(), // Already checked for key
                    None => (),
                }
            }
            {
                match self.get_mut(&key) {
                    Some(value) => value,
                    None => unreachable!(), // Already checked that key exists
                }
            }
        }
    }
}

impl Entry {
    fn insert(&mut self, path: PathBuf, file: Entry) {
        // Inner nodes in the tree should always be directories. If
        // the node is not a directory, that means we are inserting a
        // duplicate file. However, this function is only called from
        // the directory walker, which makes it impossible to observe
        // any duplicates. (And the database, after construction, is
        // always immutable.)
        match self {
            &mut Entry::Directory(ref mut entries) => {
                let mut components = path.components();
                let count = components.clone().count();
                let first = Path::new(components.next().expect("unreachable").as_os_str()).to_owned();
                let rest = components.as_path().to_owned();
                if count > 1 {
                    let mut subentry = entries.get_mut_default(first);
                    subentry.insert(rest, file);
                } else {
                    match entries.insert(first, file) {
                        Some(_) => unreachable!(), // See above
                        None => (),
                    }
                }
            }
            &mut Entry::File(_) => unreachable!()
        }
    }

    fn lookup(&self, path: &PathBuf) -> Option<&Entry> {
        match *self {
            Entry::Directory(ref entries) => {
                let mut components = path.components();
                let count = components.clone().count();
                let first = Path::new(components.next().expect("unreachable").as_os_str()).to_owned();
                let rest = components.as_path().to_owned();
                if count > 1 {
                    entries.get(&first).and_then(
                        |subentry| subentry.lookup(&rest))
                } else {
                    entries.get(&first)
                }
            }
            Entry::File(_) => unreachable!()
        }
    }
}


fn summarize<P>(path: P, expected: &Metrics, actual: &Metrics)
where
    P: AsRef<Path>,
{
    let diff = expected.size != actual.size ||
        expected.sha2 != actual.sha2 ||
        expected.sha3 != actual.sha3;
    if diff {
        let suspicious_size = expected.size > 0 && actual.size == 0;
        let suspicious_nul = expected.nul != actual.nul;
        let suspicious_nonascii = expected.nonascii != actual.nonascii;
        println!("{} differs", path.as_ref().display());
        if suspicious_size {
            println!("  suspicious: file was truncated");
        }
        if suspicious_nul {
            println!("  suspicious: original had no NUL bytes, but now does");
        }
        if suspicious_nonascii {
            println!("  suspicious: original had no non-ASCII bytes, but now does");
        }
    }
}

impl Database {
    fn insert(&mut self, path: PathBuf, entry: Entry) {
        self.0.insert(path, entry);
    }

    fn lookup(&self, path: &PathBuf) -> Option<&Entry> {
        self.0.lookup(path)
    }

    pub fn build<P>(root: P) -> Result<Database, error::Error>
    where
        P: AsRef<Path>,
    {
        let mut database = Database::default();
        for entry in WalkBuilder::new(&root).build() {
            let entry = entry?;
            if entry.file_type().map_or(false, |t| t.is_file()) {
                let metrics = compute_metrics(entry.path())?;
                let result = Entry::File(metrics);
                let short_path = if entry.path() == root.as_ref() {
                    Path::new(entry.path().file_name().expect("unreachable"))
                } else {
                    entry.path().strip_prefix(&root)?
                };
                database.insert(short_path.to_owned(), result);
            }
        }
        Ok(database)
    }

    pub fn check<P>(&self, root: P) -> Result<(), error::Error>
    where
        P: AsRef<Path>,
    {
        // FIXME: Doesn't check for missing entries
        for entry in WalkBuilder::new(&root).build() {
            let entry = entry?;
            if entry.file_type().map_or(false, |t| t.is_file()) {
                let actual = compute_metrics(entry.path())?;

                let short_path = if entry.path() == root.as_ref() {
                    Path::new(entry.path().file_name().expect("unreachable"))
                } else {
                    entry.path().strip_prefix(&root)?
                };
                let expected = self.lookup(&short_path.to_owned());
                match expected {
                    Some(&Entry::File(ref metrics)) => summarize(
                        short_path, metrics, &actual),
                    Some(&Entry::Directory(_)) => unreachable!(),
                    None => println!("{} was created", short_path.display()),
                }
            }
        }
        Ok(())
    }

    pub fn load_json<P>(path: P) -> Result<Database, error::Error>
    where
        P: AsRef<Path>
    {
        let f = File::open(path)?;
        Ok(serde_json::from_reader(f)?)
    }

    pub fn dump_json<P>(&self, path: P) -> Result<(), error::Error>
    where
        P: AsRef<Path>
    {
        let json = serde_json::to_string(self)?;
        let mut f = File::create(path)?;
        write!(f, "{}", json)?;
        Ok(())
    }

    pub fn load_cbor<P>(path: P) -> Result<Database, error::Error>
    where
        P: AsRef<Path>
    {
        let f = File::open(path)?;
        Ok(serde_cbor::from_reader(f)?)
    }

    pub fn dump_cbor<P>(&self, path: P) -> Result<(), error::Error>
    where
        P: AsRef<Path>
    {
        let cbor = serde_cbor::to_vec(self)?;
        let mut f = File::create(path)?;
        f.write_all(cbor.as_slice())?;
        Ok(())
    }
}

// impl std::fmt::Display for Database {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         for (path, entry) in self.0.iter() {
//             match entry {
//                 &Entry::File(ref hashes) => {
//                     let hash: Vec<_> = hashes.sha2.0.iter().map(
//                         |b| format!("{:02x}", b)).collect();
//                     writeln!(f, "{} {}", hash.join(""), Path::new(path).display())?
//                 }
//             }
//         }
//         Ok(())
//     }
// }
