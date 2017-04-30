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
    File(HashSums),
}

impl Default for Entry {
    fn default() -> Entry {
        Entry::Directory(BTreeMap::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashSums {
    sha2: HashSum,
    sha3: HashSum,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashSum(#[serde(with = "serde_bytes")] Vec<u8>);

#[derive(Default)]
struct Hashers {
    sha2: sha2::Sha256,
    sha3: sha3::Sha3_256,
}

impl Hashers {
    fn input(&mut self, input: &[u8]) {
        self.sha2.input(input);
        self.sha3.input(input);
    }
    fn result(self) -> HashSums {
        HashSums {
            sha2: HashSum(Vec::from(self.sha2.result().as_slice())),
            sha3: HashSum(Vec::from(self.sha3.result().as_slice())),
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
}

impl Database {
    fn insert(&mut self, path: PathBuf, entry: Entry) {
        self.0.insert(path, entry);
    }

    pub fn build<P>(path: P) -> Result<Database, error::Error>
    where
        P: AsRef<Path>
    {
        let mut database = Database::default();
        for entry in WalkBuilder::new(&path).build() {
            let entry = entry?;
            if entry.file_type().map_or(false, |t| t.is_file()) {
                let hashes = compute_hashes(entry.path())?;
                let result = Entry::File(hashes);
                let short_path = entry.path().strip_prefix(&path)?;
                database.insert(short_path.to_owned(), result);
            }
        }
        Ok(database)
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
