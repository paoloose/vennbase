use std::io::{self, prelude::*, BufReader, BufWriter};
use std::net::TcpStream;
use std::str::FromStr;

use image::ImageFormat;

use crate::db::types::MimeType;
use crate::db::vennbase::Vennbase;
use crate::features::resize::{resize_image, Dimensions, is_resizeable_format};
use crate::utils::reading::read_string_until;

const MAX_REQUEST_QUERY_LENGTH: usize = 1024;

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
                let query = match header_iter.next() {
                    Some(query) => query,
                    _ => {
                        write_to_socket!(stream, "ERROR 0\n")?;
                        continue;
                    }
                };
                match db.query_records(query) {
                    Ok(records) => {
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
                                    format!("{}\n", tags.join(",")).as_bytes()
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
                    Some(Err(_)) => None, // Just ignore invalid dimension specifiers
                    None => {
                        write_to_socket!(stream, "ERROR 0\n");
                        continue;
                    },
                };

                // When we fetch a record, we get a Take<BufReader<File>>
                match db.fetch_record_by_id(&uuid)? {
                    Some((mimetype, mut reader)) => {
                        // Stream the reader to the socket
                        let mut writer = BufWriter::new(stream);
                        let mut size = reader.limit();

                        // If we need to resize the image
                        if is_resizeable_format(mimetype) && resize_dims.is_some() {
                            let new_dimensions = resize_dims.unwrap();
                            // Load the entire image into memory
                            let mut data = Vec::with_capacity(size as usize);
                            reader.read_to_end(&mut data)?;
                            let resize_result = resize_image(
                                &data,
                                // MIMEtype should be valid at this point
                                ImageFormat::from_mime_type(mimetype.as_str()).unwrap(),
                                &new_dimensions
                            );
                            let data = match resize_result {
                                Ok(data) => data,
                                Err(_) => {
                                    writer.write_all(b"ERROR 0\n")?;
                                    continue;
                                },
                            };
                            size = data.len() as u64;

                            writer.write_all(format!("{} {}\n", mimetype, size).as_bytes())?;
                            writer.write_all(data.as_slice())?;
                        }
                        // Otherwise, send the image as it is
                        else {
                            // No processing needed, just stream the reader to the socket
                            let mut buf = [0; 1024];
                            writer.write_all(format!("{} {}\n", mimetype, size).as_bytes())?;
                            loop {
                                let bytes_read = reader.read(&mut buf)?;
                                if bytes_read == 0 { break; }
                                writer.write_all(&buf[0..bytes_read])?;
                            }
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
                let mut tags: Vec<String> = vec![];
                for _ in 0..n {
                    let (tag, _) = read_string_until(&mut reader, b'\n', MAX_REQUEST_QUERY_LENGTH)?;
                    tags.push(tag);
                }

                let mut data = Vec::with_capacity(1024);
                reader.read_to_end(&mut data)?;

                let uuid = db.save_record(&mimetype, data.as_slice())?;
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
