#![deny(elided_lifetimes_in_paths)]

pub mod db;
#[macro_use]
pub mod utils;

use std::io::{self, prelude::*, BufReader};
use std::net::{TcpListener, TcpStream};

use db::Database;
use utils::read_line_limited;

const MAX_METHOD_TYPE_SIZE: usize = 8; // max('replace', 'del', 'create')
const MAX_MIME_TYPE_LENGTH: usize = 255; // max('replace', 'del', 'create')

fn handle_connection(mut stream: TcpStream, db: &mut Database) -> io::Result<()> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(3))).unwrap();
    let mut reader = BufReader::new(&mut stream);

    let method = read_line_limited(&mut reader, MAX_METHOD_TYPE_SIZE)?;

    match method.as_str() {
        "get" => {
        },
        "new" => {
            let mut data = vec![0u8; 512];
            reader.read_to_end(&mut data)?;
            let mimetype = read_line_limited(&mut reader, MAX_MIME_TYPE_LENGTH)?;
            db.save_record(mimetype.trim_end_matches('\n'), data.as_slice());
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
        _ => {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid request type"));
        }
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:1834")?;
    let mut db = Database::from_dir("./main")?;

    for connection in listener.incoming() {
        match connection {
            Ok(conn) => {
                let _ = handle_connection(conn, &mut db);
            },
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }

    Ok(())
}
