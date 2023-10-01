use std::path::PathBuf;
use std::io::{self, prelude::*, BufReader};
use std::fs::File;

use crate::read_venn_timestamp;

use super::vennbase::{VennTimestamp, FileInformation};

// Each partition contains multiple files of the same type
pub struct Partition {
    files: Vec<FileInformation>,
    created_at: VennTimestamp,
    last_compaction: VennTimestamp
}

impl Partition {
    pub fn from_file(path: &PathBuf) -> io::Result<Self> {
        assert!(path.exists());

        if !path.is_file() {
            return Err(
                io::Error::new(io::ErrorKind::Other, "Partitions can only be files")
            )
        }

        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // NOTE: should we implement a partition name?
        // let partition_name = read_n_bytes_as_string!(&mut reader, 32).expect("Failed to read partition name");
        let created_at = read_venn_timestamp!(&mut reader).expect("Failed to read creation timestamp");
        let last_compaction = read_venn_timestamp!(&mut reader).expect("Failed to read last compaction timestamp");

        Ok(Partition {
            files: Vec::new(),
            created_at,
            last_compaction,
        })
    }
}

impl Default for Partition {
    fn default() -> Self {
        Partition {
            files: Vec::new(),
            created_at: VennTimestamp::now(),
            last_compaction: VennTimestamp::now()
        }
    }
}
