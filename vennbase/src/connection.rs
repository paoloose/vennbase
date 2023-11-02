use std::io::{self, prelude::*, BufReader, BufWriter};
use std::net::TcpStream;
use std::str::FromStr;

use crate::db::partition::StoredRecord;
use crate::db::types::MimeType;
use crate::db::vennbase::Vennbase;
use crate::features::resize::Dimensions;
use crate::utils::reading::read_string_until;

const MAX_REQUEST_QUERY_LENGTH: usize = 1024;
const MAX_RECORD_TAG_LENGTH: usize = 256;

macro_rules! write_to_socket {
    ($stream:expr, $($data:expr),*) => {{
        let mut writer = BufWriter::new($stream);
        writer.write_all(
            format!($($data),*).as_bytes()
        )
    }};
}

/// Starts the bidirectional communication with the Vennbase client.
///
/// This function only fail on unrecoverable socket errors. Input/Validation errors doesn't destroy
/// the communication with the client.
#[allow(clippy::unnecessary_unwrap)]
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
                // rest of the header
                println!("{header_iter:?}");
                let header = header_iter.collect::<Vec<_>>().join(" ");
                let query = header.as_str();
                println!("{query}");
                if query.is_empty() {
                    write_to_socket!(stream, "ERROR 0\n")?;
                    continue;
                }
                match db.query_records(query) {
                    Ok(records) => {
                        dbg!(&query);
                        if records.is_empty() {
                            write_to_socket!(stream, "OK 0\n")?;
                            continue;
                        }
                        let mut writer = BufWriter::new(stream);
                        writer.write_all(
                            format!("OK {}\n", records.len()).as_bytes()
                        )?;
                        for (mimetype, record_id) in records.iter() {
                            let tags = db.get_tags_for_record(record_id);
                            writer.write_all(
                                format!(
                                    "{record_id:?}\n{mimetype}\n{}\n",
                                    tags.len()
                                ).as_bytes()
                            )?;
                            if !tags.is_empty() {
                                writer.write_all(
                                    format!("{}\n", tags.join("\n")).as_bytes()
                                )?;
                            }
                        }
                        println!("{} record(s) queried.", records.len());
                    },
                    Err(e) => {
                        println!("Error(query): {:?}", e);
                        write_to_socket!(stream, "ERROR 0\n")?;
                    },
                }
            },
            "get" => {
                let uuid = match header_iter.next().map(uuid::Uuid::from_str) {
                    Some(Ok(id)) => id,
                    _ => {
                        write_to_socket!(stream, "ERROR 0\n")?;
                        continue;
                    }
                };
                let resize_dims: Option<Dimensions> = match header_iter.next().map(Dimensions::from_dim_str) {
                    Some(Ok(dims)) => Some(dims),
                    _ => None, // Just ignore invalid dimension specifiers
                };

                // When we fetch a record, we get a Take<BufReader<File>>
                match db.fetch_record_by_id(&uuid, &resize_dims)? {
                    Some((mimetype, mut record)) => {
                        let mut writer = BufWriter::new(stream);
                        match record {
                            StoredRecord::InDiskRecord(ref mut reader) => {
                                let size = reader.limit() as usize;
                                let mut buf = [0; 1024];
                                writer.write_all(format!("{} {}\n", mimetype, size).as_bytes())?;
                                loop {
                                    let bytes_read = reader.read(&mut buf)?;
                                    if bytes_read == 0 { break; }
                                    writer.write_all(&buf[0..bytes_read])?;
                                }
                                println!("{} bytes read.", size);
                            },
                            StoredRecord::InMemoryRecord(data) => {
                                let size = data.len();
                                writer.write_all(format!("{} {}\n", mimetype, size).as_bytes())?;
                                writer.write_all(data.as_slice())?;
                                println!("{} bytes read.", size);
                            },
                        }
                    },
                    None => {
                        write_to_socket!(stream, "NOT_FOUND 0\n")?;
                        println!("Record not found.");
                    },
                }
            },
            "save" => {
                let mimetype = match header_iter.next().map(MimeType::from) {
                    Some(Ok(mimetype)) => mimetype,
                    _ => {
                        write_to_socket!(stream, "ERROR None\n")?;
                        continue;
                    }
                };
                let n = match header_iter.next().map(str::parse::<usize>) {
                    Some(Ok(n)) => n,
                    _ => {
                        write_to_socket!(stream, "ERROR None\n")?;
                        continue;
                    },
                };
                let mut tags = vec![];
                for _ in 0..n {
                    let (tag, _) = read_string_until(&mut reader, b'\n', MAX_RECORD_TAG_LENGTH)?;
                    tags.push(tag.to_string());
                }

                let mut data = Vec::with_capacity(1024);
                reader.read_to_end(&mut data)?;

                let uuid = db.save_record(&mimetype, data.as_slice(), tags)?;
                write_to_socket!(stream, "OK {uuid}\n")?;
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
