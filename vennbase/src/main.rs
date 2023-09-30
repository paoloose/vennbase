#![deny(elided_lifetimes_in_paths)]

pub mod db;
#[macro_use]
pub mod utils;

use std::io::{self, prelude::*, BufReader};
use std::net::{TcpListener, TcpStream};

use db::Database;

static MAX_REQUEST_TYPE_SIZE: usize = 8; // max('replace', 'delete', 'create')
static MAX_MIME_TYPE_LENGTH: usize = 255; // max('replace', 'delete', 'create')

fn handle_connection(mut stream: TcpStream, db: &mut Database) -> io::Result<()> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(3))).unwrap();
    let reader = BufReader::new(&mut stream);

    let mut handle = reader.take(MAX_REQUEST_TYPE_SIZE as u64);
    let mut method = String::with_capacity(MAX_REQUEST_TYPE_SIZE);
    handle.read_line(&mut method)?;


    match method.trim_end_matches('\n') {
        "create" => {
            let mut data = vec![0u8; 512];
            handle.get_mut().read_to_end(&mut data)?;

            let mut handle = handle.get_mut().take(MAX_MIME_TYPE_LENGTH as u64);
            let mut mimetype = String::with_capacity(MAX_MIME_TYPE_LENGTH);
            handle.read_line(&mut mimetype)?;

            db.save_record(mimetype.trim_end_matches('\n'), data.as_slice());
        },
        "delete" => {
            // TODO: read id
            db.delete_record("");
        },
        "replace" => {
            let mut data = vec![0u8; 512];
            // TODO: read id
            handle.get_mut().read_to_end(&mut data)?;
            db.replace_record("", data.as_slice());
        },
        _ => {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid request type"));
        }
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6969")?;
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
