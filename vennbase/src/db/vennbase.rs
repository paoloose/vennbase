use core::panic;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, prelude::*, BufWriter};
use std::fs;
use std::path::PathBuf;

use super::partition::Partition;

#[derive(Debug)]
pub struct VennTimestamp(pub i64);

impl VennTimestamp {
    pub fn now() -> Self {
        VennTimestamp(chrono::Utc::now().timestamp_millis())
    }
}

#[derive(Eq, Hash, PartialEq, Clone)]
pub struct MimeType(String);

impl MimeType {
    pub fn from_base64_filename(path: &OsStr) -> io::Result<Self> {
        use base64::Engine;

        let filename = path.to_str().ok_or(
            io::Error::new(io::ErrorKind::InvalidData, "Invalid file name")
        )?.to_string();

        println!("decoding filename: {filename}");

        let decoded_mimetype = base64::engine::general_purpose::STANDARD_NO_PAD.decode(filename)
            .map(String::from_utf8)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(MimeType(decoded_mimetype))
    }

    pub fn to_base64_pathname(&self) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD_NO_PAD.encode(&self.0)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

// We implemented the Debug trait ourselves so that it doesn't print an unnecessary line break
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
    path: PathBuf,
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

        Ok(Vennbase { path: path.into(), partitions: HashMap::new() })
    }

    /**
     * Saves a new record the database
     */
    pub fn save_record(&mut self, mimetype: &MimeType, data: &[u8]) -> io::Result<()> {
        let partition = self.get_mut_or_create_partition(mimetype)?;
        partition.push_record(data)
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
            // Read the filename
            let filepath = entry?.path();
            let mimetype = MimeType::from_base64_filename(filepath.file_name().unwrap())?;
            println!("parsing mimetype: {:#?}", mimetype);

            partitions.insert(
                mimetype,
                Partition::from_file(filepath)?
            );
        }

        Ok(Vennbase { path: path.into(), partitions })
    }

    /// Creates a new partition for the database with the given Mime Type.
    ///
    /// Caller should ensure that the partition does not exist yet, or the whole file will be
    /// truncated.
    fn create_new_partition(&mut self, mimetype: MimeType) -> io::Result<&mut Partition> {
        let partition_path = self.path.join(mimetype.to_base64_pathname());
        assert!(!partition_path.exists());
        println!("path: {}", partition_path.to_str().unwrap());
        // File creation is done with `write: true`, `create: true`, `truncate: true`
        // So the only error we can get is either a permission error, or to a database
        // doesn't exist error. Both are fatal.
        let file = match File::create(&partition_path) {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::NotFound => todo!(),
            Err(err) if err.kind() == io::ErrorKind::PermissionDenied => todo!(),
            Err(_) => unreachable!()
        };
        let mut writer = BufWriter::new(file);

        let created_at = VennTimestamp::now();
        let last_compaction = VennTimestamp::now();

        // Not be able to write to the partition is considered fatal
        writer.write_all(created_at.0.to_le_bytes().as_slice())?;
        writer.write_all(last_compaction.0.to_le_bytes().as_slice())?;

        let new_partition = Partition::new(
            partition_path,
            HashMap::new(),
            created_at,
            last_compaction
        );

        // FIXME: we are performing two unnecessary lookups here
        self.partitions.insert(mimetype.clone(), new_partition);
        Ok(self.partitions
            .get_mut(&mimetype)
            .expect("to exist since it was just created")
        )
    }

    // FIXME: this should be optimized so that we don't need to perform two lookups
    fn get_mut_or_create_partition(&mut self, mimetype: &MimeType) -> io::Result<&mut Partition> {
        if self.partitions.contains_key(mimetype) {
            Ok(self.partitions.get_mut(mimetype).unwrap())
        }
        else {
            self.create_new_partition(mimetype.to_owned())
        }
    }
}
