use core::panic;
use std::collections::HashMap;
use std::io::{self, prelude::*, BufReader};
use std::fs::{self, File};

use crate::read_n_bytes_as_string;
use crate::read_venn_timestamp;

pub struct FileInformation {
    start: usize,
    size: usize,
}

pub struct VennTimestamp(i64);

impl VennTimestamp {
    fn now() -> Self {
        use std::time::UNIX_EPOCH;
        VennTimestamp(
            std::time::Instant::now().duration_since(UNIX_EPOCH).as_millis()
        )
    }
}

// Each partition contains multiple files of the same type
pub struct Partition {
    name: String,
    files: Vec<FileInformation>,
    created_at: VennTimestamp,
    last_compaction: VennTimestamp
}

#[derive(Eq, Hash, PartialEq)]
pub struct MimeType(String);

// A database can be seen as a universe of set theory
pub struct Database {
    path: String,
    partitions: HashMap<MimeType, Partition>
}

impl Database {
    pub fn from_dir(path: &str) -> io::Result<Database> {
        match fs::create_dir(path) {
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                return Ok(
                    Database::parse_dir_tree(path)
                        .expect("Malformed database directory")
                )
            }
            Err(e) => panic!("Couldn't create database directory: {:#?}", e),
            Ok(_) => {},
        }

        Ok(Database { path: path.to_owned(), partitions: HashMap::new() })
    }

    /**
     * Saves a new record the database
     */
    pub fn save_record(&mut self, mimetype: &str, data: &[u8]) {
        let partiition = self.partitions.get(&MimeType(mimetype.to_string()));
        let uuid = uuid::Uuid::new_v4().to_string();

        println!("Saving record with type '{}': {:#?}", mimetype, data.len())
    }

    pub fn delete_record(&mut self, id: &str) {
        println!("Deleting record with id: {}", id)
    }

    pub fn replace_record(&mut self, id: &str, data: &[u8]) {
        println!("Replacing record with id: {} with data: {:#?}", id, data.len())
    }

    fn parse_dir_tree(path: &str) -> io::Result<Database> {
        let dir = fs::read_dir(path)?;
        let mut partitions: HashMap<MimeType, Partition> = HashMap::new();

        for entry in dir {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() {
                return Err(
                    io::Error::new(io::ErrorKind::Other, "Partitions can only be files")
                )
            }

            let file = File::open(path)?;
            let mut reader = BufReader::new(file);

            let partition_name = read_n_bytes_as_string!(&mut reader, 32).expect("Failed to read partition name");
            let mimetype = read_n_bytes_as_string!(&mut reader, 255).expect("Failed to read mime type");
            let created_at = read_venn_timestamp!(&mut reader).expect("Failed to read creation timestamp");
            let last_compaction = read_venn_timestamp!(&mut reader).expect("Failed to read last compaction timestamp");

            partitions.insert(
                MimeType(mimetype),
                Partition {
                    name: partition_name,
                    files: Vec::new(), // TODO: read existing files
                    created_at,
                    last_compaction
                }
            );
        }

        Ok(Database { path: path.to_owned(), partitions })
    }

    fn create_partition(&mut self, mimetype: MimeType) {
        self.partitions.insert(
            mimetype,
            Partition {
                name: mimetype.0,
                files: Vec::new(),
                created_at: (),
                last_compaction: ()
            }
        );
    }

    fn get_or_create_partition(&mut self, mimetype: &str) -> &Partition {
        match self.partitions.get(&MimeType(mimetype.to_string())) {
            Some(partition) => partition,
            None => {
            },
        }
    }
}
