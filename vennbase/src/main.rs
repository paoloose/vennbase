#![deny(elided_lifetimes_in_paths)]

pub mod utils;
pub mod db;

use std::io::{self, prelude::*, BufReader};
use std::net::{TcpListener, TcpStream};

use crate::db::vennbase::Vennbase;
use crate::utils::reading::read_line_until;

const MAX_METHOD_TYPE_SIZE: usize = 8; // max('replace', 'del', 'create')
const MAX_MIME_TYPE_LENGTH: usize = 255; // max('replace', 'del', 'create')

fn handle_connection(mut stream: TcpStream, db: &mut Vennbase) -> io::Result<()> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(3))).unwrap();
    let mut reader = BufReader::new(&mut stream);

    let method = read_line_until(&mut reader, b' ', MAX_METHOD_TYPE_SIZE)?;

    match method.as_str() {
        "get" => {
        },
        "new" => {
            let mimetype = read_line_until(&mut reader, b'\n', MAX_MIME_TYPE_LENGTH)?;
            let mut data = vec![0u8; 512];
            reader.read_to_end(&mut data)?;
            db.save_record(&mimetype.into(), data.as_slice())?;
        },
        "del" => {
            // TODO: read id
            db.delete_record("");
        },
        "replace" => {
            let mut data = vec![0u8; 512];
            // TODO: read id
            reader.read_to_end(&mut data)?;
            db.replace_record("", data.as_slice());
        },
        m @ _ => {
            return Err(
                io::Error::new(io::ErrorKind::InvalidInput, format!("Invalid request type: {m}"))
            );
        }
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:1834")?;
    let mut db = Vennbase::from_dir("./main")?;

    for connection in listener.incoming() {
        match connection {
            Ok(conn) => {
                let result = handle_connection(conn, &mut db);
                if result.is_err() {
                    // NOTE: This is currently failing for the following reasons:
                    // - invalid utf8s
                    println!("err: {:#?}", result);
                }
            },
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }

    Ok(())
}
