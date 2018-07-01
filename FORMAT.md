# Database Format

## Goals

Goals for the format, in order from most to least important:

  * Longevity: The format should be designed to be long-lived. This
    means that the format *must* have sufficient documentation to
    permit independent implementations to be designed, without
    requiring any reverse-engineering effort.

  * Simplicity: The format should be straightforward. In the worst
    case, it should be possible to write your own parser for the
    format.

  * Compactness: The format shouldn't waste excessive space.

## Format Description

The database is a gzip-encoded JSON blob. The decompressed contents
consist of:

 1. A JSON-encoded object containing the database size and
    checksum. [JSON Schema](schema/checksum.json)

 2. The byte `0xA` (i.e. the ASCII character `\n`).

 3. A JSON-encoded object containing the database contents.
    [JSON Schema](schema/database.json)

The format is designed to be agnostic to the hash algorithm
used. Multiple algorithms may be used simultaneously. By default, the
following algorithm is used:

  * SHA2-512/256

The following algorithms are also supported:

  * BLAKE2b

## Other Formats Considered

Here are some formats under consideration:

  * Relational databases (SQL)

    SQLite is embeddable, has Rust bindings, and is able to produce a
    database file that could plausibly be used as the integrity
    database. The main disadvantages of such an approach are:

     1. Any relational database is going to support massively more
        features than we need, introducing unnecessary complexity. In
        general these formats will be optimized for efficiency of
        lookup over simplicity.

     2. It's unclear how compact the resulting database will be: both
        the format itself, and artifacts resulting from trying to
        force fundamentally tree-shaped data into a relational schema.

     3. It's unclear what kind of longevity to expect from the
        database format. Major revisions to the database software may
        introduces incompatible changes to the database format.

     4. If the database format does change, reverse-engineering the
        format and writing a parser could be non-trivial.

    Other SQL implementations such as Postgres and MySQL have an
    additional drawback: they are generally intended to be used in
    daemon mode, and do not expose the database files to the end-user
    at all.

  * Other databases

    [cdb](https://cr.yp.to/cdb.html) is a "constant database" by
    D. J. Bernstein. At a first glance, cdb looks like a good fit for
    an integrity database. It supports a one-time creation operation,
    and read-only queries. The format is compact and simple enough to
    explain in a page of text. However, the format has a number of
    drawbacks:

      1. The format has a built-in limit of 4 GB. For cdb's intended
         use cases, this is more than sufficient. However, for an
         integrity database, it is plausible that for large file
         systems, an integrity database could grow to exceed 4 GB.

      2. The format is non-hierarchical, which will inevitably result
         in duplicated data in our use case.

  * Archive formats

    The tar file format is well-known and could plausibly serve our
    use case. Basically, the integrity database would be equivalent to
    the original directory structure, but instead of story the
    contents of files, we'd store checksums and other
    metadata. However:

     1. The tar format introduces unnecessary complexity.

     2. Tar archives may waste space for the purpose we're using them,
        because the format aligns records to a certain number of
        bytes.

     3. It's unclear how you'd interact with archive. Existing tools
        are intended to expand the archive into a set of files, but in
        this case we just want to scan through the contents in
        memory. While this is probably possible, it would likely
        require you to write a custom implementation, which would (due
        to the complexity of the format) introduce the possibility of
        bugs.

  * Message formats

    Various formats (JSON, CBOR, ProtoBufs, Avro, etc.) were
    originally intended as formats to encode messages for transfer
    over the wire, but can also be used to describe data at rest.

    A key advantage of these formats is that they are well-supported
    by [Serde](https://github.com/serde-rs/serde) making it easy to
    produce robust and high-quality serializers and deserializers for
    any of these formats.

    The advantage of JSON specifically is that it is
    ubiquitous: every language can be expected to have a mature JSON
    parser available. Also, because the format is self-describing, no
    prior knowledge of the format is required. Even better, the format
    is human-readable, so you don't even need to decode it to
    understand what you are looking at. The main disadvantages of JSON
    are:

      1. The cost of being human-readable and self-describing is
         compactness. The field names of objects will be repeated, and
         binary strings (such as hashes) will need to be encoded in
         something like base64.

      2. The format must be read sequentially, and does not permit
         random access. This is not an issue for the features
         described above, but could prove problematic for certain
         plausible extensions to the functionality of the tool.

    There are other formats that are less ubiquitous but improve on
    the first point by offering compact binary encodings.

    CBOR is a self-describing binary format that is relatively simple
    and has an [official standard](http://cbor.io/). Because it is
    self-describing, there will still be some inefficiency in the
    encoding, especially of objects.

    [Apache Avro](https://avro.apache.org/) is a binary format in
    which all documents are accompanied by a schema. Thus a document
    can be interpreted without any external knowledge of the schema,
    but avoids the repetition of traditional self-describing
    formats. However, the format is not standardized and it is unclear
    what stability guarantees are offered. Overall, Avro is less
    popular, so there is additional risk of the format being
    unsupported in the future.

  * Custom format

    It would also be possible to design a custom format. Version
    control systems like Git and Mercurial essentially do
    this. However, those projects have the advantage of being popular,
    so there is interest in having multiple robust implementations of
    their formats. For a new tool, however, it would be better to use
    existing, well-known formats, as these are more likely to provide
    the desired longevity.
