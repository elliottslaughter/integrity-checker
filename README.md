# Backup Integrity Checker

This tool is (or will be) an integrity checker for backups. The vast
majority of the functionality is currently missing, but the intention
is to do the following:

  * Given a directory, the tools should walk recursively walk files in
    the directory and construct a database of metadata (hashes, size,
    timestamps, etc.) of all. The database itself should of course be
    checksummed as well.

  * Given two databases, or a database and a directory, the tool
    should iterate the entries and print a *helpful* summary of the
    differences between them. For example, the tool should highlight
    suspicious patterns, such as files which got truncated (had
    non-zero size, and now have zero size) or have other patterns that
    could indicate corruption (e.g. the presence of NUL bytes, if the
    file originally had none).

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
    useful, the tool has to be very good at summarizing differences.)

## Format

See the [format description](FORMAT.md).

## FAQ

  * Isn't this better served by existing tools? ZFS, Tarsnap,
    etc. should never corrupt your data.

    Well, it depends. In general, defense in depth is good, even with
    relatively trustworthy tools such as ZFS and Tarsnap. Also, in the
    continuous sync use case, even with backups, it can often be
    difficult to be assured that you haven't been subject to silent
    data corruption. This tool can be part of a larger toolkit for
    ensuring the validity of long-term storage.

## TODO

  * Set returncode on check/diff: 0 (no changes), 1 (changes), 2
    (suspicious changes), negative (failure); or something similar
  * Checksum the database itself (i.e. encode the database in memory,
    compute checksum(s), and write the database prefixed by size and
    checksum(s))
  * Consider whether compression of the database should be included
  * Traverse files in parallel when building the initial database
  * Measure performance and see if any of the major components (e.g. the
    checksums) are CPU-bound and can be made to run any faster
  * Check the results on real-world backups and see if anything can be done
    to surface useful data while minimizing false positives
  * Rewrite check subcommand to report results interactively, instead of
    synchronously building an entire database in memory
  * Review the output of check/diff and consider if it can be made
    more helpful
  * Unit/integration tests
