use std::io::{self, prelude::*, BufReader, BufWriter};
use std::net::TcpStream;
use std::str::FromStr;

use image::ImageFormat;

use crate::db::vennbase::Vennbase;
use crate::features::resize::{resize_image, Dimensions, is_resizeable_format};
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
                let query = header_iter.next();
                if query.is_none() {
                    write_to_socket!(stream, "No query provided\n")?;
                    continue;
                }
                match db.query_record(query.unwrap()) {
                    Ok(records) => {
                        let mut writer = BufWriter::new(stream);
                        if records.is_empty() {
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
                let mut resize_dims: Option<Dimensions> = None;
                let resize_str = match header_iter.next() {
                    Some(resize) => resize,
                    None => {
                        write_to_socket!(stream, "No query provided\n")?;
                        continue;
                    },
                };
                let id = match header_iter.next() {
                    Some(id) => {
                        resize_dims = Some(Dimensions::from_dim_str(resize_str).map_err(|_| {
                            io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "Invalid resize dimensions"
                            )
                        })?); // FIXME: send message instead of quitting
                        // FIXME: resize_dims must be valid
                        id
                    },
                    None => resize_str
                };

                let uuid = match uuid::Uuid::from_str(id) {
                    Ok(id) => id,
                    Err(_) => {
                        write_to_socket!(stream, "Invalid UUID: '{}'\n", id)?;
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
                            let dims = resize_dims.unwrap();
                            // Load the entire image into memory
                            let mut data = Vec::with_capacity(size as usize);
                            reader.read_to_end(&mut data)?;
                            // MIMEtype should be valid at this point
                            let data = resize_image(
                                &data,
                                ImageFormat::from_mime_type(mimetype.as_str()).unwrap(),
                                &dims
                            ).unwrap();
                            size = data.len() as u64;

                            writer.write_all(
                                format!("{} {}\n", mimetype, size).as_bytes()
                            )?;
                            writer.write_all(data.as_slice())?;
                        }
                        // Otherwise, send the image as it is
                        else {
                            // No processing needed, just stream the reader to the socket
                            let mut buf = [0; 1024];
                            writer.write_all(
                                format!("{} {}\n", mimetype, size).as_bytes()
                            )?;
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
