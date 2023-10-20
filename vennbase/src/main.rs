#![deny(elided_lifetimes_in_paths)]
pub mod utils;
pub mod db;
pub mod query;
pub mod pool;
pub mod connection;
pub mod features;

use std::io;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use crate::db::vennbase::Vennbase;
use crate::pool::ThreadPool;
use crate::connection::handle_connection;

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:1834")?;
    println!("Listening on port 1834 🐢\n");
    let db = Arc::new(Mutex::new(Vennbase::from_dir("./venndb")?));
    let pool = ThreadPool::with_same_workers_as_cpus().unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(conn) => {
                let db = Arc::clone(&db);
                pool.run(move || {
                    let mut db = db.lock().unwrap();
                    let result = handle_connection(&conn, &mut db);
                    if result.is_err() {
                        // NOTE: This is currently failing for the following reasons:
                        // - invalid utf8s
                        // red color
                        println!("\u{001b}[31m[ERR]\u{001b}[0m {:?}", result.unwrap_err());
                    }
                });
            },
            Err(e) => {
                println!("\u{001b}[31m[ERR]\u{001b}[0m {:?}", e);
            }
        }
    }

    Ok(())
}
