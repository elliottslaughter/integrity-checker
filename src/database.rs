use std::collections::BTreeMap;
use std::cmp::Ordering;
use std::default::Default;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use digest::Digest;
#[cfg(feature = "blake2b")]
use digest::VariableOutput;
use ignore::{WalkBuilder, WalkState};
use time;

use serde_json;

use sha2;
#[cfg(feature = "blake2b")]
use blake2;

use base64;
use error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabaseChecksum {
    sha2: HashSum,
    #[cfg(feature = "blake2b")]
    blake2: HashSum,
    size: u64,
}

impl From<Metrics> for DatabaseChecksum {
    fn from(metrics: Metrics) -> Self {
        DatabaseChecksum {
            sha2: metrics.sha2,
            #[cfg(feature = "blake2b")]
            blake2: metrics.blake2,
            size: metrics.size,
        }
    }
}

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
    #[cfg(feature = "blake2b")]
    blake2: HashSum,
    size: u64,      // File size
    nul: bool,      // Does the file contain a NUL byte?
    nonascii: bool, // Does the file contain non-ASCII bytes?
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashSum(#[serde(with = "base64")] Vec<u8>);

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
    sha2: sha2::Sha512Trunc256,
    #[cfg(feature = "blake2b")]
    blake2: blake2::Blake2b,
    size: EngineSize,
    nul: EngineNul,
    nonascii: EngineNonascii,
}

impl Engines {
    fn input(&mut self, input: &[u8]) {
        self.sha2.input(input);
        #[cfg(feature = "blake2b")]
        self.blake2.input(input);
        self.size.input(input);
        self.nul.input(input);
        self.nonascii.input(input);
    }
    fn result(self) -> Metrics {
        #[cfg(feature = "blake2b")]
        let mut buffer = [0; 32];
        Metrics {
            sha2: HashSum(Vec::from(self.sha2.result().as_slice())),
            #[cfg(feature = "blake2b")]
            blake2: HashSum(
                Vec::from(self.blake2.variable_result(&mut buffer).unwrap())),
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
    fn get_default(&mut self, key: K) -> &mut V;
}

impl<K, V> BTreeMapExt<K, V> for BTreeMap<K, V>
where
    K: Ord + Clone,
    V: Default,
{
    fn get_default(&mut self, key: K) -> &mut V {
        self.entry(key).or_insert_with(|| V::default())
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
                    let mut subentry = entries.get_default(first);
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

#[derive(Debug)]
pub enum EntryDiff {
    Directory(BTreeMap<PathBuf, EntryDiff>, DirectoryDiff),
    File(MetricsDiff),
    KindChanged,
}

#[derive(Debug)]
pub struct DirectoryDiff {
    added: u64,
    removed: u64,
    changed: u64,
    unchanged: u64,
}

#[derive(Debug)]
pub struct MetricsDiff {
    changed_content: bool,
    zeroed: bool,
    changed_nul: bool,
    changed_nonascii: bool,
}

#[derive(Debug, PartialEq)]
pub enum DiffSummary {
    NoChanges,
    Changes,
    Suspicious,
}

impl EntryDiff {
    fn show_diff(&self, path: &PathBuf, depth: usize) {
        match *self {
            EntryDiff::Directory(ref entries, ref diff) => {
                if diff.changed > 0 || diff.added > 0 || diff.removed > 0 {
                    println!("{}{}: {} changed, {} added, {} removed, {} unchanged",
                             "| ".repeat(depth),
                             path.display(),
                             diff.changed,
                             diff.added,
                             diff.removed,
                             diff.unchanged);
                    for (key, entry) in entries.iter() {
                        entry.show_diff(key, depth+1);
                    }
                }
            }
            EntryDiff::File(ref diff) => {
                if diff.zeroed || diff.changed_nul || diff.changed_nonascii {
                    println!("{}{} changed",
                             "| ".repeat(depth),
                             path.display());
                    if diff.zeroed {
                        println!("{}> suspicious: file was truncated",
                                 "##".repeat(depth));
                    }
                    if diff.changed_nul {
                        println!("{}> suspicious: original had no NUL bytes, but now does",
                                 "##".repeat(depth));
                    }
                    if diff.changed_nonascii {
                        println!("{}> suspicious: original had no non-ASCII bytes, but now does",
                                 "##".repeat(depth));
                    }
                }
            }
            EntryDiff::KindChanged => {
            }
        }
    }

    fn summarize_diff(&self) -> DiffSummary {
        match *self {
            EntryDiff::Directory(ref entries, ref diff) => {
                let initial =
                    if diff.changed > 0 || diff.added > 0 || diff.removed > 0 {
                        DiffSummary::Changes
                    } else {
                        DiffSummary::NoChanges
                    };
                entries
                    .values()
                    .map(|x| x.summarize_diff())
                    .fold(initial, |acc, x| acc.meet(x))
            }
            EntryDiff::File(ref diff) => {
                if diff.zeroed || diff.changed_nul || diff.changed_nonascii {
                    DiffSummary::Suspicious
                } else if diff.changed_content {
                    DiffSummary::Changes
                } else {
                    DiffSummary::NoChanges
                }
            }
            EntryDiff::KindChanged => {
                DiffSummary::Changes
            }
        }
    }
}

impl DiffSummary {
    fn meet(self, other: DiffSummary) -> DiffSummary {
        if self == DiffSummary::Suspicious || other == DiffSummary::Suspicious {
            DiffSummary::Suspicious
        } else if self == DiffSummary::Changes || other == DiffSummary::Changes {
            DiffSummary::Changes
        } else {
            DiffSummary::NoChanges
        }
    }
}

impl Entry {
    fn diff(&self, other: &Entry) -> EntryDiff {
        match (self, other) {
            (&Entry::Directory(ref old), &Entry::Directory(ref new)) => {
                let mut entries = BTreeMap::default();
                let mut added = 0;
                let mut removed = 0;
                let mut changed = 0;
                let mut unchanged = 0;

                let mut old_iter = old.iter();
                let mut new_iter = new.iter();
                let mut old_entry = old_iter.next();
                let mut new_entry = new_iter.next();
                while old_entry.is_some() && new_entry.is_some() {
                    let (old_key, old_value) = old_entry.unwrap();
                    let (new_key, new_value) = new_entry.unwrap();
                    match old_key.cmp(new_key) {
                        Ordering::Less => {
                            removed += 1;
                            old_entry = old_iter.next();
                        }
                        Ordering::Greater => {
                            added += 1;
                            new_entry = new_iter.next();
                        }
                        Ordering::Equal => {
                            let diff = old_value.diff(new_value);
                            match diff {
                                EntryDiff::Directory(_, ref stats) => {
                                    added += stats.added;
                                    removed += stats.removed;
                                    changed += stats.changed;
                                    unchanged += stats.unchanged;
                                }
                                EntryDiff::File(ref stats) => {
                                    if stats.changed_content {
                                        changed += 1;
                                    } else {
                                        unchanged += 1;
                                    }
                                }
                                EntryDiff::KindChanged => {
                                    changed += 1;
                                }
                            }
                            entries.insert(old_key.clone(), diff);
                            old_entry = old_iter.next();
                            new_entry = new_iter.next();
                        }
                    }
                }
                removed += old_iter.count() as u64;
                added += new_iter.count() as u64;
                EntryDiff::Directory(
                    entries,
                    DirectoryDiff { added, removed, changed, unchanged })
            },
            (&Entry::File(ref old), &Entry::File(ref new)) => {
                let changed = old.size != new.size;
                let changed = changed || old.sha2 != new.sha2;
                #[cfg(feature = "blake2b")]
                let changed = changed || old.blake2 != new.blake2;
                EntryDiff::File(
                    MetricsDiff {
                        changed_content: changed,
                        zeroed: old.size > 0 && new.size == 0,
                        changed_nul: old.nul != new.nul,
                        changed_nonascii: old.nonascii != new.nonascii,
                    }
                )
            },
            (_, _) => EntryDiff::KindChanged,
        }
    }
}

impl Database {
    fn insert(&mut self, path: PathBuf, entry: Entry) {
        self.0.insert(path, entry);
    }

    pub fn lookup(&self, path: &PathBuf) -> Option<&Entry> {
        self.0.lookup(path)
    }

    pub fn diff(&self, other: &Database) -> EntryDiff {
        self.0.diff(&other.0)
    }

    pub fn build<P>(root: P, verbose: bool, threads: usize) -> Result<Database, error::Error>
    where
        P: AsRef<Path>,
    {
        let total_bytes = Arc::new(Mutex::new(0));
        let database = Arc::new(Mutex::new(Database::default()));
        let start_time_ns = time::precise_time_ns();

        let parallel = threads > 1;
        if parallel {
            WalkBuilder::new(&root).threads(threads).build_parallel().run(|| {
                let total_bytes = total_bytes.clone();
                let database = database.clone();
                let root = root.as_ref().to_owned();
                Box::new(move |entry| {
                    let entry = entry.unwrap(); // ?
                    if entry.file_type().map_or(false, |t| t.is_file()) {
                        let metrics = compute_metrics(entry.path()).unwrap(); // ?
                        *total_bytes.lock().unwrap() += metrics.size;
                        let result = Entry::File(metrics);
                        let short_path = if entry.path() == root {
                            Path::new(entry.path().file_name().expect("unreachable"))
                        } else {
                            entry.path().strip_prefix(&root).unwrap() // ?
                        };
                        database.lock().unwrap().insert(short_path.to_owned(), result);
                    }
                    WalkState::Continue
                })
            });
        } else {
            let ref mut total_bytes = *total_bytes.lock().unwrap();
            let ref mut database = *database.lock().unwrap();
            for entry in WalkBuilder::new(&root).build() {
                let entry = entry?;
                if entry.file_type().map_or(false, |t| t.is_file()) {
                    let metrics = compute_metrics(entry.path())?;
                    *total_bytes += metrics.size;
                    let result = Entry::File(metrics);
                    let short_path = if entry.path() == root.as_ref() {
                        Path::new(entry.path().file_name().expect("unreachable"))
                    } else {
                        entry.path().strip_prefix(&root)?
                    };
                    database.insert(short_path.to_owned(), result);
                }
            }
        }
        let stop_time_ns = time::precise_time_ns();
        if verbose {
            let total_bytes = *total_bytes.lock().unwrap();
            println!("Database::build took {:.3} seconds on {} threads, read {} bytes, {:.1} MB/s",
                     (stop_time_ns - start_time_ns) as f64/1e9,
                     threads,
                     total_bytes,
                     total_bytes as f64/((stop_time_ns - start_time_ns) as f64/1e3));
        }
        let ref database = *database.lock().unwrap();
        Ok(database.clone())
    }

    pub fn show_diff(&self, other: &Database) -> DiffSummary {
        let diff = self.diff(other);
        diff.show_diff(&Path::new(".").to_owned(), 0);
        diff.summarize_diff()
    }

    pub fn check<P>(&self, root: P, threads: usize) -> Result<DiffSummary, error::Error>
    where
        P: AsRef<Path>,
    {
        // FIXME: This is non-interactive, but vastly simply than
        // trying to implement the same functionality interactively.
        let other = Database::build(root, false, threads)?;
        Ok(self.show_diff(&other))
    }

    pub fn load_json<P>(path: P) -> Result<Database, error::Error>
    where
        P: AsRef<Path>
    {
        // Read entire file contents to memory
        let mut f = File::open(path)?;
        let mut bytes = Vec::new();
        f.read_to_end(&mut bytes)?;
        let bytes = bytes;

        // Find position of separator \n (byte 0x0a)
        let index = bytes.iter().position(|&x| x == 0x0a).unwrap();

        // Decode expected checksums
        let expected : DatabaseChecksum =
            serde_json::from_slice(&bytes[..index])?;

        // Compute actual checksums of database
        let mut engines = Engines::default();
        engines.input(&bytes[index+1..]);
        let actual: DatabaseChecksum = engines.result().into();

        if expected != actual {
            return Err(error::Error::ChecksumMismatch);
        }

        // Continue decoding database
        Ok(serde_json::from_slice(&bytes[index+1..])?)
    }

    pub fn dump_json<P>(&self, path: P) -> Result<(), error::Error>
    where
        P: AsRef<Path>
    {
        // Important: The encoded JSON **must not** contain a \n character,
        // or else the format will break

        // Generate JSON-encoded database
        let db_json = serde_json::to_vec(self)?;

        // Compute checksums of encoded JSON
        let mut engines = Engines::default();
        engines.input(&db_json[..]);
        let checksum: DatabaseChecksum = engines.result().into();
        let checksum_json = serde_json::to_vec(&checksum)?;

        // Write checksum and database separated by \n
        let mut f = File::create(path)?;
        f.write(&checksum_json[..])?;
        f.write(&vec![0x0a][..])?; // Use \n (byte 0x0a) as separator
        f.write(&db_json)?;
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
