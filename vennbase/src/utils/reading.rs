use std::io::{self, prelude::*, BufReader, BufRead};

#[macro_export]
macro_rules! read_venn_timestamp {
    ($reader: expr) => {{
        use $crate::db::vennbase::VennTimestamp;
        let mut creation_time = [0u8; 8];
        $reader.read_exact(&mut creation_time)
            .map(|_| VennTimestamp(i64::from_be_bytes(creation_time)))
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

/**
 * Read a line from a buffer until either an stop byte is found or the limit 'n'
 * was reached.
 */
pub fn read_line_until<S>(reader: &mut BufReader<S>, stop: u8, n: usize) -> io::Result<String>
where S: Read + Write {
    let mut line = Vec::with_capacity(n);
    let mut handle = reader.take(n as u64);
    handle.read_until(stop, &mut line)?;
    Ok(
        String::from_utf8(line)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
            .trim_end_matches(stop as char)
            .to_string()
    )
}
