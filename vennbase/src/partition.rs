use std::collections::HashMap;
use std::path::PathBuf;
use std::io::{self, prelude::*, BufReader, BufWriter, SeekFrom};
use std::fs::{File, OpenOptions};

use crate::{read_venn_timestamp, read_u64, read_n_bytes};
use crate::db::types::VennTimestamp;

#[derive(Debug)]
pub struct RecordInformation {
    is_active: bool,
    start: u64,
    size: u64,
}

pub type BufferedRecord = io::Take<BufReader<File>>;

// Each partition contains multiple files of the same type
#[derive(Debug)]
pub struct Partition {
    file_path: PathBuf,
    records: HashMap<uuid::Uuid, RecordInformation>,
    #[allow(dead_code)]
    created_at: VennTimestamp,
    #[allow(dead_code)]
    last_compaction: VennTimestamp, // TODO: implement compaction
    next_start: u64,
}

const HASHMAP_INITIAL_CAPACITY: usize = 128;
// Each reacord header is 25 bytes long, setting the BufReader capacity to 32
// improves the performance from ~25.49786663s to ~340.529302ms.
const BUFFREADER_CAPACITY: usize = 32;

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
        let mut reader = BufReader::with_capacity(BUFFREADER_CAPACITY, file);

        println!("  from {file_path:?}");

        // NOTE: should we implement a partition name?
        let created_at = read_venn_timestamp!(&mut reader)?;
        let last_compaction = read_venn_timestamp!(&mut reader)?;

        // Start reading the list of records
        // (skipping the partition header, since we already read it)
        // NOTE: we use a loop since we don't exactly know how many records there are
        let mut next_record_start = PARTITION_HEADER_BYTES_OFFSET;
        let mut records: HashMap<uuid::Uuid, RecordInformation> = HashMap::with_capacity(
            HASHMAP_INITIAL_CAPACITY
        );

        loop {
            let mut flags: [u8; 1] = [0];
            match reader.read_exact(&mut flags) {
                Ok(()) => (),
                Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(err) => return Err(err)
            }
            let is_active = flags[0] & 0b10000000 != 0;
            let record_id = uuid::Uuid::from_bytes(
                read_n_bytes!(&mut reader, RECORD_ID_SIZE_BYTES as usize)?
            );
            let record_size = read_u64!(&mut reader)?;

            next_record_start += RECORD_HEADER_SIZE_BYTES;
            records.insert(
                record_id,
                RecordInformation {
                    is_active,
                    start: next_record_start,
                    size: record_size
                }
            );
            // Skip the {record_size} bytes of data
            reader.seek(SeekFrom::Current(record_size as i64))?;
            next_record_start += record_size;
        }
        println!("  with {} record(s)", records.len());

        // TODO: load the records from the file
        Ok(Partition {
            file_path,
            records,
            created_at,
            last_compaction,
            next_start: next_record_start + RECORD_HEADER_SIZE_BYTES
        })
    }

    /// Returns the number of records in the partition.
    ///
    /// This considers both active and inactive records.
    pub fn records_len(&self) -> usize {
        self.records.len()
    }

    /// Must be called when a new partition on the disk has been created.
    ///
    /// This sets the `next_start` pointing to the first record content data (skipping the header).
    pub fn new(
        file_path: PathBuf,
        files: HashMap<uuid::Uuid, RecordInformation>,
        created_at: VennTimestamp,
        last_compaction: VennTimestamp
    ) -> Self {
        Partition {
            file_path,
            records: files,
            created_at,
            last_compaction,
            next_start: PARTITION_HEADER_BYTES_OFFSET + RECORD_HEADER_SIZE_BYTES
        }
    }

    pub fn push_record(&mut self, data: &[u8]) -> io::Result<uuid::Uuid> {
        let uuid = uuid::Uuid::new_v4();
        // FIXME: should we move the writer to the struct itself?
        let file = OpenOptions::new()
            .append(true)
            .open(&self.file_path)?;

        let mut writer = BufWriter::new(file);
        writer.write_all(&[1 << 7])?;
        writer.write_all(uuid.as_bytes())?;
        writer.write_all((data.len() as u64).to_le_bytes().as_slice())?;
        writer.write_all(data)?;

        self.records.insert(
            uuid,
            RecordInformation {
                is_active: true,
                start: self.next_start,
                size: data.len() as u64
            }
        );

        self.next_start += data.len() as u64 + RECORD_HEADER_SIZE_BYTES;

        Ok(uuid)
    }

    pub fn get_record_information(&self, record_id: &uuid::Uuid) -> Option<&RecordInformation> {
        self.records.get(record_id)
    }

    pub fn fetch_record(&self, record_id: &uuid::Uuid) -> io::Result<Option<BufferedRecord>> {
        match self.records.get(record_id) {
            Some(record_info) => {
                let file = File::open(&self.file_path)?;
                let mut reader = BufReader::new(file);

                reader.seek(SeekFrom::Start(record_info.start))?;
                Ok(Some(reader.take(record_info.size)))
            },
            None => {
                Ok(None)
            },
        }
    }

    pub fn iter_active_records(&self) -> impl Iterator<Item=(&uuid::Uuid, &RecordInformation)> {
        self.records
            .iter()
            .filter(|(_, record)| record.is_active)
    }
}
