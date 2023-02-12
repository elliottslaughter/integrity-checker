use std::collections::BTreeMap;
use std::cmp::Ordering;
use std::default::Default;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use digest::{FixedOutput, consts::U32, Digest};
use ignore::{WalkBuilder, WalkState};
use time;

use serde_json;

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;

use sha2::Sha512_256;
use blake2;

use crate::base64;
use crate::error;

type Blake2b32 = blake2::Blake2b<U32>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Features {
    pub sha2: bool,
    pub blake2b: bool,
}

impl Default for Features {
    fn default() -> Features {
        Features {
            sha2: true,
            blake2b: false,
        }
    }
}

impl Features {
    fn infer_from_database_checksum(checksum: &DatabaseChecksum) -> Features {
        Features {
            sha2: checksum.sha2.is_some(),
            blake2b: checksum.blake2b.is_some(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabaseChecksum {
    #[serde(rename = "sha2-512/256")]
    #[serde(skip_serializing_if = "Option::is_none")]
    sha2: Option<HashSum>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blake2b: Option<HashSum>,
    size: u64,
}

impl DatabaseChecksum {
    fn diff(&self, new: &Self) -> bool {
        let changed = self.size != new.size;
        let changed = changed || (self.sha2.is_some() && new.sha2.is_some() && self.sha2 != new.sha2);
        let changed = changed ||
            (self.blake2b.is_some() && new.blake2b.is_some() && self.blake2b != new.blake2b);
        changed
    }
}

impl From<Metrics> for DatabaseChecksum {
    fn from(metrics: Metrics) -> Self {
        DatabaseChecksum {
            sha2: metrics.sha2,
            blake2b: metrics.blake2b,
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
    #[serde(rename = "sha2-512/256")]
    #[serde(skip_serializing_if = "Option::is_none")]
    sha2: Option<HashSum>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blake2b: Option<HashSum>,
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

struct Engines {
    sha2: Option<Sha512_256>,
    blake2b: Option<Blake2b32>,
    size: EngineSize,
    nul: EngineNul,
    nonascii: EngineNonascii,
}

impl Engines {
    fn new(features: Features) -> Engines {
        Engines {
            sha2: if features.sha2 {
                Some(Sha512_256::default())
            } else {
                None
            },
            blake2b: if features.blake2b {
                Some(Blake2b32::new())
            } else {
                None
            },
            size: EngineSize::default(),
            nul: EngineNul::default(),
            nonascii: EngineNonascii::default(),
         }
    }
}

impl Engines {
    fn input(&mut self, input: &[u8]) {
        self.sha2.iter_mut().for_each(|e| e.update(input));
        self.blake2b.iter_mut().for_each(|e| e.update(input));
        self.size.input(input);
        self.nul.input(input);
        self.nonascii.input(input);
    }
    fn result(self) -> Metrics {
        Metrics {
            sha2: self.sha2.map(|e| HashSum(Vec::from(e.finalize_fixed().as_slice()))),
            blake2b: self.blake2b.map(|e| HashSum(
                Vec::from(e.finalize().as_slice()))),
            size: self.size.result(),
            nul: self.nul.result(),
            nonascii: self.nonascii.result(),
        }
    }
}

fn compute_metrics(path: impl AsRef<Path>, features: Features) -> Result<Metrics, error::Error> {
    let mut f = File::open(path)?;

    let mut engines = Engines::new(features);

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
            Entry::Directory(entries) => {
                let mut components = path.components();
                let count = components.clone().count();
                let first = Path::new(components.next().expect("unreachable").as_os_str()).to_owned();
                let rest = components.as_path().to_owned();
                if count > 1 {
                    let subentry = entries.get_default(first);
                    subentry.insert(rest, file);
                } else {
                    match entries.insert(first, file) {
                        Some(_) => unreachable!(), // See above
                        None => (),
                    }
                }
            }
            Entry::File(_) => unreachable!()
        }
    }

    fn lookup(&self, path: &PathBuf) -> Option<&Entry> {
        match self {
            Entry::Directory(entries) => {
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
        match self {
            EntryDiff::Directory(entries, diff) => {
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
            EntryDiff::File(diff) => {
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
        match self {
            EntryDiff::Directory(entries, diff) => {
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
            EntryDiff::File(diff) => {
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
            (Entry::Directory(old), Entry::Directory(new)) => {
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
            (Entry::File(old), Entry::File(new)) => {
                let changed = old.size != new.size;
                let changed = changed ||
                    (old.sha2.is_some() && new.sha2.is_some() && old.sha2 != new.sha2);
                let changed = changed ||
                    (old.blake2b.is_some() && new.blake2b.is_some() && old.blake2b != new.blake2b);
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

const SEP : u8 = 0x0a; // separator \n (byte 0x0a) used in JSON encoding

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

    pub fn build(
        root: impl AsRef<Path>,
        features: Features,
        threads: usize,
        verbose: bool,
    ) -> Result<Database, error::Error> {
        let total_bytes = Arc::new(Mutex::new(0));
        let database = Arc::new(Mutex::new(Database::default()));
        let start_time = time::Instant::now();

        let parallel = threads > 1;
        if parallel {
            WalkBuilder::new(&root).threads(threads).build_parallel().run(|| {
                let total_bytes = total_bytes.clone();
                let database = database.clone();
                let root = root.as_ref().to_owned();
                Box::new(move |entry| {
                    let entry = entry.unwrap(); // ?
                    if entry.file_type().map_or(false, |t| t.is_file()) {
                        let metrics = compute_metrics(entry.path(), features).unwrap(); // ?
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
                    let metrics = compute_metrics(entry.path(), features)?;
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
        let elapsed = start_time.elapsed().as_seconds_f64();
        if verbose {
            let total_bytes = *total_bytes.lock().unwrap();
            println!("Database::build took {:.3} seconds on {} threads, read {} bytes, {:.1} MB/s",
                     elapsed,
                     threads,
                     total_bytes,
                     total_bytes as f64/elapsed/1e6);
        }
        let ref database = *database.lock().unwrap();
        Ok(database.clone())
    }

    pub fn show_diff(&self, other: &Database) -> DiffSummary {
        let diff = self.diff(other);
        diff.show_diff(&Path::new(".").to_owned(), 0);
        diff.summarize_diff()
    }

    pub fn check(
        &self,
        root: impl AsRef<Path>,
        features: Features,
        threads: usize
    ) -> Result<DiffSummary, error::Error> {
        // FIXME: This is non-interactive, but vastly more simple than
        // trying to implement the same functionality interactively.
        let other = Database::build(root, features, threads, false)?;
        Ok(self.show_diff(&other))
    }

    pub fn load_json(r: impl Read) -> Result<Database, error::Error> {
        // Read entire contents to memory
        let mut d = GzDecoder::new(r);

        let mut bytes = Vec::new();
        d.read_to_end(&mut bytes)?;
        let bytes = bytes;

        // Find position of separator
        let index = match bytes.iter().position(|&x| x == SEP) {
            Some(x) => x,
            None => return Err(error::Error::ParseError),
        };

        // Decode expected checksums
        let expected : DatabaseChecksum =
            serde_json::from_slice(&bytes[..index])?;
        let features = Features::infer_from_database_checksum(&expected);

        // Compute actual checksums of database
        let mut engines = Engines::new(features);
        engines.input(&bytes[index+1..]);
        let actual: DatabaseChecksum = engines.result().into();

        if expected.diff(&actual) {
            return Err(error::Error::ChecksumMismatch);
        }

        // Continue decoding database
        Ok(serde_json::from_slice(&bytes[index+1..])?)
    }

    pub fn dump_json<W>(&self, w: W, features: Features) -> Result<W, error::Error>
    where
        W: Write
    {
        // Important: The encoded JSON **must not** contain the separator,
        // or else the format will break

        // Generate JSON-encoded database
        let db_json = serde_json::to_vec(self)?;

        // Compute checksums of encoded JSON
        let mut engines = Engines::new(features);
        engines.input(&db_json[..]);
        let checksum: DatabaseChecksum = engines.result().into();
        let checksum_json = serde_json::to_vec(&checksum)?;

        // Make sure encoded JSON does not include separator
        assert!(!checksum_json.contains(&SEP));

        // Write checksum, separator and database
        let mut e = GzEncoder::new(w, Compression::best());
        e.write_all(&checksum_json[..])?;
        e.write_all(&vec![SEP][..])?;
        e.write_all(&db_json)?;
        Ok(e.finish()?)
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
