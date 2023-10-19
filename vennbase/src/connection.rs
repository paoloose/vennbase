use std::io::{self, prelude::*, BufReader, BufWriter};
use std::net::TcpStream;
use std::str::FromStr;

use crate::db::vennbase::Vennbase;
use crate::utils::reading::read_string_until;

const MAX_MIME_TYPE_LENGTH: usize = 255;
const MAX_REQUEST_QUERY_LENGTH: usize = 1024;

macro_rules! write_to_socket {
    ($stream:expr, $($data:expr),*) => {{
        let mut writer = BufWriter::new($stream);
        writer.write_all(
            format!($($data),*).as_bytes()
        )
    }};
}

pub fn handle_connection(stream: &TcpStream, db: &mut Vennbase) -> io::Result<()> {
    let mut reader = BufReader::new(stream);
    // Each loop iteration represents a request
    loop {
        let (header, eof) = read_string_until(&mut reader, b'\n', MAX_REQUEST_QUERY_LENGTH)?;
        if eof { break; }
        if header.is_empty() { continue; }

        let mut header_iter = header.split(' ');
        let method = header_iter.next().unwrap_or_default();

        println!("\u{001b}[32m[REQ]\u{001b}[0m {header}");

        match method {
            "query" => {
                let query = header_iter.next();
                if query.is_none() {
                    write_to_socket!(stream, "No query provided\n")?;
                    continue;
                }
                match db.query_record(query.unwrap()) {
                    Ok(records) => {
                        let mut writer = BufWriter::new(stream);
                        if records.len() == 0 {
                            write_to_socket!(stream, "\n")?;
                            continue;
                        }
                        for record in records.iter() {
                            writer.write_all(
                                format!("{:#?}\n", record).as_bytes()
                            )?;
                        }
                        println!("{} record(s) queried.", records.len());
                    },
                    Err(e) => println!("Error(get): {:?}", e),
                }
            },
            "get" => {
                let id = header_iter.next();
                if id.is_none() {
                    write_to_socket!(stream, "No query provided\n")?;
                    continue;
                }
                let uuid = match uuid::Uuid::from_str(id.unwrap()) {
                    Ok(id) => id,
                    Err(_) => {
                        write_to_socket!(stream, "Invalid UUID: '{}'\n", id.unwrap())?;
                        continue;
                    },
                };
                // When we fetch a record, we get a Take<BufReader<File>>
                match db.fetch_record_by_id(&uuid)? {
                    Some((mimetype, mut reader)) => {
                        let size = reader.limit();
                        // Stream the reader to the socket
                        let mut writer = BufWriter::new(stream);
                        let mut buf = [0; 512];

                        writer.write_all(
                            format!("{} {}\n", mimetype, size).as_bytes()
                        )?;
                        loop {
                            let bytes_read = reader.read(&mut buf)?;
                            if bytes_read == 0 { break; }
                            writer.write_all(&buf[0..bytes_read])?;
                        }
                        println!("{} bytes read.", size);
                    },
                    None => {
                        write_to_socket!(stream, "NOT_FOUND 0\n")?;
                        println!("Record not found.");
                    },
                }
            },
            "save" => {
                let mimetype = header_iter.next().unwrap_or_default();
                if mimetype.is_empty() {
                    write_to_socket!(stream, "No mimetype provided\n")?;
                    continue;
                }
                if mimetype.len() > MAX_MIME_TYPE_LENGTH {
                    write_to_socket!(stream, "Mimetype too long\n")?;
                    continue;
                }

                let mut data = Vec::with_capacity(512);
                reader.read_to_end(&mut data)?;

                let uuid = db.save_record(&mimetype.into(), data.as_slice())?;
                write_to_socket!(stream, "{uuid}")?;
                println!("Saving record {uuid} with len {:#?}", data.len());
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
                write_to_socket!(stream, "Unknown method: '{}'\n", other)?;
            }
        }
    }

    Ok(())
}
