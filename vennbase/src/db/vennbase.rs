use core::panic;
use std::collections::HashMap;
use std::io;
use std::fs;
use std::path::PathBuf;

use chrono::format;

use super::partition::Partition;

#[derive(Debug)]
pub struct FileInformation {
    start: usize,
    size: usize,
}

#[derive(Debug)]
pub struct VennTimestamp(pub i64);

impl VennTimestamp {
    pub fn now() -> Self {
        use chrono::prelude::*;
        let now = Utc::now();
        VennTimestamp(now.timestamp_millis())
    }
}

#[derive(Eq, Hash, PartialEq)]
pub struct MimeType(String);

impl MimeType {
    fn from_pathname(path: &PathBuf) -> io::Result<Self> {
        let path = path.to_str().ok_or(
            io::Error::new(io::ErrorKind::InvalidData, "Invalid path name")
        )?.to_string();

        Ok(MimeType(path))
    }
}

impl std::fmt::Debug for MimeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("MimeType({})", self.0).as_str())
    }
}

impl From<String> for MimeType {
    fn from(s: String) -> Self {
        MimeType(s)
    }
}

// A database can be seen as a universe of set theory
pub struct Vennbase {
    path: String,
    partitions: HashMap<MimeType, Partition>
}

impl Vennbase {
    pub fn from_dir(path: &str) -> io::Result<Vennbase> {
        match fs::create_dir(path) {
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                return Ok(
                    Vennbase::parse_dir_tree(path)
                        .expect("Malformed database directory")
                )
            }
            Err(e) => panic!("Couldn't create database directory: {:#?}", e),
            Ok(_) => {},
        }

        Ok(Vennbase { path: path.to_owned(), partitions: HashMap::new() })
    }

    /**
     * Saves a new record the database
     */
    pub fn save_record(&mut self, mimetype: &str, data: &[u8]) {
        let partition = self.get_or_create_partition(mimetype);
        let uuid = uuid::Uuid::new_v4().to_string();

        println!("{:#?}", self.partitions);

        println!("Saving record {uuid} with type '{}': {:#?}", mimetype, data.len())
    }

    pub fn delete_record(&mut self, id: &str) {
        println!("Deleting record with id: {}", id)
    }

    pub fn replace_record(&mut self, id: &str, data: &[u8]) {
        println!("Replacing record with id: {} with data: {:#?}", id, data.len())
    }

    fn parse_dir_tree(path: &str) -> io::Result<Vennbase> {
        let dir = fs::read_dir(path)?;
        let mut partitions: HashMap<MimeType, Partition> = HashMap::new();

        for entry in dir {
            let entry = entry?;
            let path = entry.path();
            let mimetype = MimeType::from_pathname(&path)?;

            partitions.insert(
                mimetype,
                Partition::from_file(&path)?
            );
        }

        Ok(Vennbase { path: path.to_owned(), partitions })
    }

    fn get_or_create_partition(&mut self, mimetype: &str) -> &Partition {
        let mimetype = MimeType(mimetype.to_string());
        self.partitions.entry(mimetype).or_insert(Partition::default())
    }
}
