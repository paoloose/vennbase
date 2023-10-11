use std::io::{self, prelude::*, BufReader};
use std::net::TcpStream;
use crate::utils::reading::read_string_until;
use crate::db::vennbase::Vennbase;

const MAX_METHOD_TYPE_SIZE: usize = 8; // max('replace', 'del', 'create')
const MAX_MIME_TYPE_LENGTH: usize = 255;
const MAX_QUERY_INPUT_LENGTH: usize = 1024;

pub fn handle_connection(mut stream: TcpStream, db: &mut Vennbase) -> io::Result<()> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(3))).unwrap();
    let mut reader = BufReader::new(&mut stream);

    let method = read_string_until(&mut reader, b' ', MAX_METHOD_TYPE_SIZE)?;

    match method.as_str() {
        "get" => {
            let query = read_string_until(&mut reader, b'\n', MAX_QUERY_INPUT_LENGTH)?;
            match db.query_record(query.as_str()) {
                Ok(records) => {
                    println!("{records:#?}");
                    println!("{} record(s) queried.", records.len());
                },
                Err(e) => println!("Error(get): {:?}", e),
            }
        },
        "new" => {
            let mimetype = read_string_until(&mut reader, b'\n', MAX_MIME_TYPE_LENGTH)?;
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
            return Err(
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Invalid request type: {other}")
                )
            );
        }
    }

    Ok(())
}
