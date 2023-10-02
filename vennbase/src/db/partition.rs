use std::path::PathBuf;
use std::io::{self, prelude::*, BufReader, BufWriter};
use std::fs::{File, OpenOptions};

use crate::read_venn_timestamp;

use super::vennbase::VennTimestamp;

#[derive(Debug)]
pub struct FileInformation {
    start: usize,
    size: usize,
}

// Each partition contains multiple files of the same type
#[derive(Debug)]
pub struct Partition {
    file_path: PathBuf,
    records: Vec<FileInformation>,
    created_at: VennTimestamp,
    last_compaction: VennTimestamp
}

impl Partition {
    /// Loads the partition data from an existing file_path.
    ///
    /// Caller MUST ensure that the file_path exists before calling this function.
    ///
    /// This function doesn't parse the Mime Type from the file name, it is the caller's
    /// responsibility to ensure that the file_path is correct.
    pub fn from_file_path(file_path: PathBuf) -> io::Result<Self> {
        assert!(file_path.exists());

        if !file_path.is_file() {
            return Err(
                io::Error::new(io::ErrorKind::Other, "Partitions can only be files")
            )
        }

        let file = File::open(&file_path)?;
        let mut reader = BufReader::new(file);

        // NOTE: should we implement a partition name?
        // let partition_name = read_n_bytes_as_string!(&mut reader, 32).expect("Failed to read partition name");
        let created_at = read_venn_timestamp!(&mut reader).expect("Failed to read creation timestamp");
        let last_compaction = read_venn_timestamp!(&mut reader).expect("Failed to read last compaction timestamp");

        // TODO: load the records from the file
        Ok(Partition {
            file_path,
            records: Vec::new(),
            created_at,
            last_compaction,
        })
    }

    pub fn new(file_path: PathBuf, files: Vec<FileInformation>, created_at: VennTimestamp, last_compaction: VennTimestamp) -> Self {
        Partition {
            file_path,
            records: files,
            created_at,
            last_compaction
        }
    }

    pub fn push_record(&self, data: &[u8]) -> io::Result<()> {
        // FIXME: should we move the writer to the struct itself?
        let file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(&self.file_path)?;

        let mut writer = BufWriter::new(file);
        writer.write_all(data)
    }
}
