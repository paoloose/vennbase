use std::io::{self, prelude::*, BufReader, BufRead};


#[macro_export]
macro_rules! read_venn_timestamp {
    ($reader: expr) => {{
        use $crate::db::VennTimestamp;
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

pub fn read_line_limited<S: Read + Write>(reader: &mut BufReader<S>, n: usize) -> io::Result<String> {
    let mut line = String::with_capacity(n);
    let mut handle = reader.take(n as u64);
    handle.read_line(&mut line)?;
    Ok(line.trim_end_matches('\n').to_owned())
}
