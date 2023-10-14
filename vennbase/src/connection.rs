use std::io::{self, prelude::*, BufReader, BufWriter};
use std::net::TcpStream;

use crate::db::vennbase::Vennbase;
use crate::utils::reading::read_string_until;

const MAX_REQUEST_QUERY_LENGTH: usize = 1024;

macro_rules! write_to_socket {
    ($stream:expr, $($data:expr),*) => {{
        let mut writer = BufWriter::new($stream);
        writer.write_all(
            format!($($data),*).as_bytes()
        )?;
    }};
}

pub fn handle_connection(stream: &TcpStream, db: &mut Vennbase) -> io::Result<()> {
    let mut reader = BufReader::new(stream);

    // Each loop iteration represents a request
    loop {
        let (header, eof) = read_string_until(&mut reader, b'\n', MAX_REQUEST_QUERY_LENGTH)?;
        if eof { break; }

        let mut header_iter = header.split(' ');
        let method = header_iter.next().unwrap_or_default();

        match method {
            "query" => {
                let query = header_iter.next();
                if query.is_none() {
                    write_to_socket!(stream, "No query provided\n");
                    continue;
                }
                match db.query_record(query.unwrap()) {
                    Ok(records) => {
                        println!("{records:#?}");
                        println!("{} record(s) queried.", records.len());
                    },
                    Err(e) => println!("Error(get): {:?}", e),
                }
            },
            "save" => {
                let mimetype = header_iter.next().unwrap_or_default();
                dbg!(&mimetype);
                let mut data = Vec::with_capacity(512);
                reader.read_to_end(&mut data)?;
                db.save_record(&mimetype.into(), data.as_slice())?;
            },
            "del" => {
                // TODO: read id
                db.delete_record("");
            },
            "replace" => {
                let mut data = Vec::with_capacity(512);
                // TODO: read id
                reader.read_to_end(&mut data)?;
                db.replace_record("", data.as_slice());
            },
            other => {
                write_to_socket!(stream, "Unknown method: '{}'\n", other);
            }
        }
    }

    Ok(())
}
