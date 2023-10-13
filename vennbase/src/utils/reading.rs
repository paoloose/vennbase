use std::io::{self, prelude::*, BufReader, BufRead};

#[macro_export]
macro_rules! read_venn_timestamp {
    ($reader: expr) => {{
        use $crate::db::types::VennTimestamp;
        let mut creation_time = [0u8; 8];
        $reader.read_exact(&mut creation_time)
            .map(|_| VennTimestamp(i64::from_le_bytes(creation_time)))
    }};
}

#[macro_export]
macro_rules! read_u64 {
    ($reader: expr) => {{
        let mut creation_time = [0u8; 8];
        $reader.read_exact(&mut creation_time)
            .map(|_| u64::from_le_bytes(creation_time))
    }};
}

#[macro_export]
macro_rules! read_n_bytes_as_string {
    ($reader: expr, $n: expr) => {{
        let mut buffer = [0u8; $n];
        $reader.read_exact(&mut buffer)
            .map(|_| String::from_utf8_lossy(&buffer).to_string())
    }};
}

#[macro_export]
macro_rules! read_n_bytes {
    ($reader: expr, $n: expr) => {{
        let mut buffer = [0u8; $n];
        $reader.read_exact(&mut buffer)
            .map(|_| buffer)
    }};
}

/**
 * Read a line from a buffer until either an stop byte is found or the limit 'stop'
 * was reached.
 *
 * The difference between the usual read_until method is that this wraps the reader on
 * a reader.take adaptaer so you can set a limit on the number of bytes read.
 */
pub fn read_string_until<S>(reader: &mut BufReader<S>, stop: u8, max_length: usize) -> io::Result<String>
where S: Read {
    let mut line = Vec::with_capacity(max_length);
    let mut handle = reader.take(max_length as u64);
    handle.read_until(stop, &mut line)?;
    Ok(
        String::from_utf8(line)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
            .trim_end_matches(stop as char)
            .to_string()
    )
}
