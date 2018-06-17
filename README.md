# Backup Integrity Checker [![Build Status](https://travis-ci.org/elliottslaughter/integrity-checker.svg?branch=master)](https://travis-ci.org/elliottslaughter/integrity-checker) [![Build Status](https://ci.appveyor.com/api/projects/status/w3mn421fl8l6r95a/branch/master?svg=true)](https://ci.appveyor.com/project/elliottslaughter/integrity-checker/branch/master)

This tool is an integrity checker for backups and filesystems:

  * Given a directory, the tools constructs a database of metadata
    (hashes, sizes, timestamps, etc.) of the contents. The database
    itself is of course checksummed as well.

  * Given two databases, or a database and a directory, the tool
    iterates the entries and prints a *helpful* summary of the
    differences between them. For example, the tool highlights
    suspicious patterns, such as files which got truncated (had
    non-zero size, and now have zero size) or have other patterns that
    could indicate corruption (e.g. the presence of NUL bytes, if the
    file originally had none). Surfacing useful data while minimizing
    false positives is an ongoing effort.

Here are a couple sample use cases:

  * Backup integrity checking: Record a database when you make a
    backup. When restoring the backup, compare against the database to
    make sure the backup restore function has worked properly. (Or
    better, perform this check periodically to ensure that the backups
    are functioning properly.)

  * Continuous sync sanity checking: Suppose you use a tool like
    Dropbox. In theory, your files are "backed up" on a continuous
    basis. In practice, you have no assurance that the tool isn't
    modifying files behind your back. By recording databases
    periodically, you can sanity check that directories that shouldn't
    change often are in fact not changing. (Note: For this to be
    useful, the tool has to be very good at minimizing false positives.)

    This also applies to any live filesystem. Consider that a typical
    user will maintain continuity of data across possibly decades of
    hardware and filesystem upgrades. Every transition is an
    opportunity for silent data corruption. Better to be safe than
    sorry.

The tool is designed around an especially stable database format so
that if something were to happen, it would be relatively
straightforward to recover the contained metadata.

## Performance

Corpus: [Linux 4.16.7
source](https://cdn.kernel.org/pub/linux/kernel/v4.x/linux-4.16.7.tar.xz)
(4403 directories, 62872 files, 890 MiB)

Machine: 2016 MacBook Pro 2.7 GHz Quad-Core i7

| Configuration          | Time (s) | BW (MiB/s) |
|:---------------------- | --------:| ----------:|
| No Hash                |   0.8832 |     1007.7 |
| SHA2-512/256           |   1.3128 |      677.9 |
| Blake2b                |   1.3034 |      682.8 |
| SHA2-512/256 + Blake2b |   1.8119 |      491.2 |

## Format

See the [format description](FORMAT.md).

## FAQ

  * Isn't this better served by existing tools? ZFS, Tarsnap,
    etc. should never corrupt your data.

    Well, it depends. Not all users have access to a filesystem that
    checksums file contents, or to a machine with ECC RAM, and even
    the ones that do may experience filesystem bugs. In general,
    defense in depth is good, even with relatively trustworthy tools
    such as ZFS and Tarsnap. Also, in the continuous sync use case,
    even with backups, it can often be difficult to be assured that
    you haven't been subject to silent data corruption. This tool can
    be part of a larger toolkit for ensuring the validity of long-term
    storage.

## TODO

  * Measure performance and see if any of the major components (e.g. the
    checksums) are CPU-bound and can be made to run any faster
  * Check the results on real-world backups and see if anything can be done
    to surface useful data while minimizing false positives
  * Rewrite check subcommand to report results interactively, instead of
    synchronously building an entire database in memory
  * Review the output of check/diff and consider if it can be made
    more helpful
  * Decide what metadata, if any, to save. Ideas:
      * [X] Contains NUL bytes
      * [X] Contains non-ASCII bytes
      * [ ] Is encodable as UTF-8 or other formats
      * [ ] Line endings (certain VCS tools like to munge these)
      * [ ] Is a symlink (Dropbox likes to forget this one)
      * [ ] Has extended attributes or resource forks or other unusual features
      * [ ] File name capitalization differs (might indicate trouble with a case-insensitive file system)
      * [ ] Multiple files with names that differ only in capitalization (might indicate trouble with a case-sensitive file system)
      * [ ] Differs in permissions (might indicate trouble with file system that doesn't track permissions)
  * Unit/integration tests
      * Test top-level command workflows
      * Test that database checksums work (i.e. modification to database or checksum results in error)
      * Test long-term stability of the format (i.e. older databases can be read and used)
  * Add a `-v` flag that shows verbose diffs
