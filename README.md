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

## FAQ

  * Isn't this better served by existing tools? ZFS, Tarsnap,
    etc. should never corrupt your data.

    Well, it depends. In general, defense in depth is good, even with
    relatively trustworthy tools such as ZFS and Tarsnap. Also, in the
    continuous sync use case, even with backups, it can often be
    difficult to be assured that you haven't been subject to silent
    data corruption. This tool can be part of a larger toolkit for
    ensuring the validity of long-term storage.
