use core::panic;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{self, prelude::*, BufWriter};
use std::fs;
use std::ops::Index;
use std::path::PathBuf;

use logic_parser::parsing::ASTNode;

use crate::db::types::VennTimestamp;
use crate::db::types::MimeType;
use crate::db::partition::Partition;
use crate::query::parse_query;

// A database can be seen as a universe of set theory
pub struct Vennbase {
    path: PathBuf,
    partitions: HashMap<MimeType, Partition>
}

#[derive(Debug)]
pub struct VennbaseError(String);

impl Vennbase {
    pub fn from_dir(path: &str) -> io::Result<Vennbase> {
        match fs::create_dir(path) {
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                return Ok(
                    Vennbase::parse_dir_tree(path)
                        .expect("Malformed database directory")
                )
            }
            Err(e) => panic!("Couldn't create database directory: {:#?}", e),
            Ok(_) => {},
        }

        Ok(Vennbase { path: path.into(), partitions: HashMap::new() })
    }

    /**
     * Saves a new record the database
     */
    pub fn save_record(&mut self, mimetype: &MimeType, data: &[u8]) -> io::Result<()> {
        let partition = self.get_mut_or_create_partition(mimetype)?;
        partition.push_record(data)
    }

    pub fn delete_record(&mut self, id: &str) {
        println!("Deleting record with id: {}", id)
    }

    pub fn replace_record(&mut self, id: &str, data: &[u8]) {
        println!("Replacing record with id: {} with data: {:#?}", id, data.len())
    }

    pub fn query_record(&self, query: &str) -> Result<(), VennbaseError> {
        let parsed_query = parse_query(query)
            .map_err(|_| VennbaseError("Invalid query".into()))?;

        println!("received query: {:#?}", parsed_query);
        // FIXME: this need to be optimized. maybe using the Shunting yard algorithm?
        // https://en.wikipedia.org/wiki/Shunting_yard_algorithm
        let partition_i = 0;

        fn evaluate(node: ASTNode) -> Result<bool, ()> {
            match node {
                ASTNode::Not { operand } => Ok(!evaluate(*operand)?),
                ASTNode::And { left, right } => Ok(evaluate(*left)? && evaluate(*right)?),
                ASTNode::Or { left, right } => Ok(evaluate(*left)? || evaluate(*right)?),
                ASTNode::Implies { left, right } => Ok(!evaluate(*left)? || evaluate(*right)?),
                ASTNode::IfAndOnlyIf { left, right } => Ok(evaluate(*left)? == evaluate(*right)?),
                ASTNode::Literal { value } => Ok(value),
                ASTNode::Identifier { name: expression } => {
                    // get index of ':'
                    let colon_i = expression.find(':').ok_or(())?;
                    // FIXME: improve error granularity
                    if colon_i == 0 || colon_i > expression.len() || expression.rfind(':').unwrap() != colon_i {
                        return Err(());
                    }
                    // Due to the checks, `filter` and `name` must be valid strings at this point
                    let (filter, value) = expression.split_at(colon_i);
                    match filter {
                        "tag" => {},
                        "mime" => {},
                        "id" => {}
                        _ => return Err(())
                    }
                    Ok(true)
                },
            }
        }

        for partition in &self.partitions {
        }

        Ok(())
    }

    fn parse_dir_tree(path: &str) -> io::Result<Vennbase> {
        let dir = fs::read_dir(path)?;
        let mut partitions: HashMap<MimeType, Partition> = HashMap::new();

        for entry in dir {
            // Read the filename
            let filepath = entry?.path();
            let mimetype = MimeType::from_base64_filename(filepath.file_name().unwrap())?;
            println!("Found partition: {:#?}", mimetype);

            partitions.insert(
                mimetype,
                Partition::from_file(filepath)?
            );
        }

        Ok(Vennbase { path: path.into(), partitions })
    }

    /// Creates a new partition for the database with the given Mime Type.
    ///
    /// Caller should ensure that the partition does not exist yet, or the whole file will be
    /// truncated.
    fn create_new_partition(&mut self, mimetype: MimeType) -> io::Result<&mut Partition> {
        let partition_path = self.path.join(mimetype.to_base64_pathname());
        assert!(!partition_path.exists());
        println!("New partition: {}", partition_path.to_str().unwrap());
        // File creation is done with `write: true`, `create: true`, `truncate: true`
        // So the only error we can get is either a permission error, or to a database
        // doesn't exist error. Both are fatal.
        let file = match File::create(&partition_path) {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::NotFound => todo!(),
            Err(err) if err.kind() == io::ErrorKind::PermissionDenied => todo!(),
            Err(_) => unreachable!()
        };
        let mut writer = BufWriter::new(file);

        let created_at = VennTimestamp::now();
        let last_compaction = VennTimestamp::now();

        // Not be able to write to the partition is considered fatal
        writer.write_all(created_at.0.to_le_bytes().as_slice())?;
        writer.write_all(last_compaction.0.to_le_bytes().as_slice())?;

        let new_partition = Partition::new(
            partition_path,
            HashMap::new(),
            created_at,
            last_compaction
        );

        // FIXME: we are performing two unnecessary lookups here
        self.partitions.insert(mimetype.clone(), new_partition);
        Ok(self.partitions
            .get_mut(&mimetype)
            .expect("to exist since it was just created")
        )
    }

    // FIXME: this should be optimized so that we don't need to perform two lookups
    fn get_mut_or_create_partition(&mut self, mimetype: &MimeType) -> io::Result<&mut Partition> {
        if self.partitions.contains_key(mimetype) {
            Ok(self.partitions.get_mut(mimetype).unwrap())
        }
        else {
            self.create_new_partition(mimetype.to_owned())
        }
    }

    // fn evaluate_query() -> Result<(), VennbaseError> {
    //
    // }
}
