# Vennbase

A (**pretty much WIP**) disk-efficient multimedia database that partitions data by content type.

## Database and partitions

A `.vennbase` database file contains information about the database with the
following structure:

| Length   | Content                                           |
| -------- | ------------------------------------------------- |
| 16 bytes | A version string with the form `vennbase@version` |
| 32 bytes | The Database name                                 |
| 54 bits  | Database creation [timestamp](#timestamps)        |
|          |                                                   |

Database partitions are represented as `.vennpart` files in the same directory as the `.vennbase`
database. Each partition represents a different content type of multimedia.

| Length    | Content                                            |
| --------- | -------------------------------------------------- |
| 32 bytes  | The partition name                                 |
| 255 bytes | The partition MIME type                            |
| 54 bits   | Partition creation [timestamp](#timestamps)        |
| 54 bits   | Last partition compaction [timestamp](#timestamps) |
| —         | List of record structures                          |
|           |                                                    |

Where each record structure has the following structure:

| Length    | Content                                                  |
| --------- | -------------------------------------------------------- |
| 1 bit     | A bit indicating whether this record is active or not.   |
| 7 bits    | Record bit flags (reserved for future use; must be zero) |
| 64 bits   | Record length (`l`) in bytes                             |
| `l` bytes | The actual record data                                   |
|           |                                                          |

Inactive records will be deleted in the next database compaction.

Please note:

- All Vennbase data is formatted in little-endian.
- All Vennbase strings are UTF-8 encoded.

## Vennbase data types

### Timestamps

Vennbase timestamps follow the
[ECMAScript Time Values](https://262.ecma-international.org/5.1/#sec-15.9.1.1)
specification, measured in milliseconds since the UNIX epoch (54 bits).

This allows timestamps to be safely converted to a JavaScript `Date` object by simply
calling `new Date(timestamp)`. On other languages, Vennbase timestamps can be converted
to a UNIX timestamp by dividing the value by 1000 and handling overflows accordingly.

## To do

- [ ] Implement in-memory caching with `shared_buffers` like PostgreSQL. Currently, all
    key-value lookups are in-memory, which can cause performance issues with large
    databases.
