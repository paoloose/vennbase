# Vennbase

A (**pretty much WIP**) disk-efficient multimedia database that partitions data by content type.

The following features are for documentation purpose, and they may not be implemented yet.

## Querying the database

```bash
# Let the 'venn' alias be:
alias venn="nc 127.0.0.1 1834 -qv"
```

### Creating a record with `save`

```plain
save <content-type> len=<len> tags=[...<tags>]
<binary-data>
```

**Examples:**

Store a new image in the database.

```bash
venn <<< $'save image/png len=692521 tags=['pink' 'anime' 'rock']\n' < ./data/image.png
```

Storing an image without tags.

```bash
img_len=$(wc -c < ./data/image.png)
venn <<< $'save image/png len=${img_len}\n' < ./data/image.png
```

### Querying records with `query`

Vennbase queries are written in a custom query language, similar to the logic
expressions you already know:

```plain
query skip=<n> limit=<m>
<query>
```

**Examples:**

Retrieving the images and videos with tags pink and anime.

```bash
venn <<< $'query (mime:image/* && tag:anime) || (mime:video/* && !tag:anime)'
```

```bash
venn <<< $'query skip=20 limit=10 (tag:'pink' || tag:'anime') && (mime:image/* || mime:video/*)'
```

### Fetching records with `get`

```plain
get <id>
```

**Examples:**

Downloading a record with ID `f81d4fae-7dec-11d0-a765-00a0c91e6bf6`.

```bash
venn <<< $'fetch f81d4fae-7dec-11d0-a765-00a0c91e6bf6' > ./data/image.png
```

## Database and partitions

A `.vennbase` database file contains information about the database with the
following structure:

| Length   | Content                                           |
| -------- | ------------------------------------------------- |
| 16 bytes | A version string with the form `vennbase@version` |
| 32 bytes | The Database name                                 |
| 64 bits  | Database creation [timestamp](#timestamps)        |
|          |                                                   |

Database partitions are represented as `.vennpart` files in the same directory as the `.vennbase`
database. Each partition represents a different content type of multimedia.

| Length    | Content                                            |
| --------- | -------------------------------------------------- |
| 64 bits   | Partition creation [timestamp](#timestamps)        |
| 64 bits   | Last partition compaction [timestamp](#timestamps) |
| —         | List of record structures                          |
|           |                                                    |

Where each record structure has the following structure:

| Length    | Content                                                  |
| --------- | -------------------------------------------------------- |
| 1 bit     | A bit indicating whether this record is active or not.   |
| 7 bits    | Record bit flags (reserved for future use; must be zero) |
| 16 bytes  | The ID (UUID v4) of the record                           |
| 64 bits   | Unsigned record length (`l`) in bytes                    |
| `l` bytes | The actual record data                                   |
|           |                                                          |

Inactive records will be deleted in the next database compaction.

Please note:

- All Vennbase data is stored in little-endian format.
- All Vennbase strings are UTF-8 encoded.

## Vennbase data types

### Timestamps

Vennbase timestamps are signed 64-bit integers representing the number of milliseconds
since the UNIX epoch (UTC). Be aware of this when converting Venn timestamps to other formats
like the one from the
[ECMAScript specification](https://262.ecma-international.org/5.1/#sec-15.9.1.1),
which uses 54 bits instead.

Vennbase uses the [chrono](https://docs.rs/chrono/latest/chrono/) trait to generate its timestamps.
For more information, refer to its documentation.

```rs
impl VennTimestamp {
    pub fn now() -> Self {
        VennTimestamp(chrono::Utc::now().timestamp_millis())
    }
}
```

## To do

- [ ] Implement in-memory caching with `shared_buffers` like PostgreSQL. Currently, all
    key-value lookups are in-memory, which can cause performance issues with large
    databases.
