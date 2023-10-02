use std::collections::HashMap;
use std::path::PathBuf;
use std::io::{self, prelude::*, BufReader, BufWriter, SeekFrom};
use std::fs::{File, OpenOptions};

use crate::{read_venn_timestamp, read_u64, read_n_bytes};

use super::vennbase::VennTimestamp;

#[derive(Debug)]
pub struct FileInformation {
    is_active: bool,
    start: u64,
    size: u64,
}

// Each partition contains multiple files of the same type
#[derive(Debug)]
pub struct Partition {
    file_path: PathBuf,
    records: HashMap<uuid::Uuid, FileInformation>,
    created_at: VennTimestamp,
    last_compaction: VennTimestamp,
    next_start: u64,
}

const TIMESTAMP_SIZE_BYTES: u64 = 8;
const PARTITION_HEADER_BYTES_OFFSET: u64 = TIMESTAMP_SIZE_BYTES + TIMESTAMP_SIZE_BYTES;

const RECORD_ID_SIZE_BYTES: u64 = 16;
const RECORD_BIT_FLAGS_SIZE_BYTES: u64 = 1;
const RECORD_DATA_LENGTH_SIZE_BYTES: u64 = 8;

const RECORD_HEADER_SIZE_BYTES: u64 =
    RECORD_BIT_FLAGS_SIZE_BYTES +
    RECORD_ID_SIZE_BYTES +
    RECORD_DATA_LENGTH_SIZE_BYTES;

impl Partition {
    /// Loads the partition data from an existing file_path.
    ///
    /// Caller MUST ensure that the file_path exists before calling this function.
    ///
    /// This function doesn't parse the Mime Type from the file name, it is the caller's
    /// responsibility to ensure that the file_path is correct.
    pub fn from_file(file_path: PathBuf) -> io::Result<Self> {
        assert!(file_path.exists());

        if !file_path.is_file() {
            return Err(
                io::Error::new(io::ErrorKind::Other, "Partitions can only be files")
            )
        }

        let file = File::open(&file_path)?;
        let mut reader = BufReader::new(file);
        println!("  Reading partition from {:#?}", file_path);

        // NOTE: should we implement a partition name?
        // let partition_name = read_n_bytes_as_string!(&mut reader, 32).expect("Failed to read partition name");
        let created_at = read_venn_timestamp!(&mut reader).expect("Failed to read creation timestamp");
        let last_compaction = read_venn_timestamp!(&mut reader).expect("Failed to read last compaction timestamp");

        // Start reading the list of records
        // NOTE: we use a loop since we don't exactly know how many records there are
        let mut next_record_start = PARTITION_HEADER_BYTES_OFFSET;
        let mut records: HashMap<uuid::Uuid, FileInformation> = HashMap::new();

        loop {
            let mut flags: [u8; 1] = [0];
            match reader.read_exact(&mut flags) {
                Ok(()) => (),
                Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(err) => return Err(err)
            }
            let is_active = flags[0] & 0b10000000 != 0;
            let record_id = uuid::Uuid::from_bytes(read_n_bytes!(&mut reader, 16)?);
            let record_size = read_u64!(&mut reader)?;

            // Skip the {record_size} bytes of data
            reader.seek(SeekFrom::Start(next_record_start + RECORD_HEADER_SIZE_BYTES + record_size))?;

            records.insert(
                record_id,
                FileInformation {
                    is_active,
                    start: next_record_start,
                    size: record_size
                }
            );
            next_record_start += RECORD_HEADER_SIZE_BYTES + record_size;
        }

        println!("  with {} record(s)", records.len());

        // TODO: load the records from the file
        Ok(Partition {
            file_path,
            records,
            created_at,
            last_compaction,
            next_start: next_record_start
        })
    }

    pub fn new(
        file_path: PathBuf,
        files: HashMap<uuid::Uuid, FileInformation>,
        created_at: VennTimestamp,
        last_compaction: VennTimestamp
    ) -> Self {
        Partition {
            file_path,
            records: files,
            created_at,
            last_compaction,
            next_start: PARTITION_HEADER_BYTES_OFFSET
        }
    }

    pub fn push_record(&mut self, data: &[u8]) -> io::Result<()> {
        let uuid = uuid::Uuid::new_v4();
        // FIXME: should we move the writer to the struct itself?
        let file = OpenOptions::new()
            .append(true)
            .open(&self.file_path)?;

        println!("Saving record {uuid} with len {:#?}", data.len());
        let mut writer = BufWriter::new(file);
        writer.write(&[1 << 7])?;
        writer.write(uuid.as_bytes())?;
        writer.write((data.len() as u64).to_le_bytes().as_slice())?;
        writer.write_all(data)?;

        self.records.insert(
            uuid,
            FileInformation {
                is_active: true,
                start: self.next_start,
                size: data.len() as u64
            }
        );
        self.next_start += (data.len() + 9) as u64; // 1 byte for the header, 8 bytes for the size

        Ok(())
    }
}
