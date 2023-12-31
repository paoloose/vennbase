use std::collections::HashMap;
use core::panic;
use std::fs::{self, File};
use std::io::{self, prelude::*, BufWriter, BufReader};
use std::path::PathBuf;

use crate::db::types::{VennTimestamp, MimeType};
use crate::db::partition::{Partition, StoredRecord};
use crate::features::fast_querying::InvertedIndexMap;
use crate::features::resize::{Dimensions, is_resizable_format, resize_image};
use crate::query::parse_query;

use image::ImageFormat;
use logic_parser::parsing::ASTNode;

/// A venbase database instance.
///
/// Conceptually, you can think of a database as a universe from Set Theory,
/// partitioned by content type, where each element of a partitions is called a record.
pub struct Vennbase {
    path: PathBuf,
    partitions: HashMap<MimeType, Partition>,
    tags: InvertedIndexMap,
}

#[derive(Debug)]
pub struct VennbaseError(String);

impl Vennbase {
    /// Parses an existing vennbase database directory
    pub fn from_dir(path: &str) -> io::Result<Vennbase> {
        match fs::create_dir(path) {
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                let tree = Vennbase::parse_dir_tree(path).expect("Malformed database directory");
                Ok(tree)
            }
            Err(e) => panic!("Couldn't create database directory: {:#?}", e),
            Ok(_) => {
                // create a .map file
                let map_path = PathBuf::from(path).join(".map");
                let file = File::create(map_path.clone())?;
                let mut writer = BufWriter::new(file);
                let tags_map = InvertedIndexMap { map: HashMap::new(), path: map_path.clone() };
                let serialized_map = serde_json::to_string(&tags_map).unwrap();
                writer.write_all(serialized_map.as_bytes())?;

                Ok(Vennbase {
                    path: path.into(),
                    partitions: HashMap::new(),
                    tags: tags_map
                })
            },
        }
    }

    /// Saves a new record the database and returns its UUID.
    ///
    /// If the partition for the given mimetype doesn't exist, it will be created.
    pub fn save_record(
        &mut self,
        mimetype: &MimeType, data: &[u8],
        tags: Vec<String>
    ) -> io::Result<uuid::Uuid> {
        let partition = self.get_mut_or_create_partition(mimetype)?;
        partition.push_record(data).map(|uuid| {
            for t in tags {
                self.tags.add_tag(t.as_str(), uuid);
            }
            uuid
        })
    }

    pub fn delete_record(&mut self, id: &str) {
        unimplemented!("Deleting record with id: {}", id);
    }

    pub fn replace_record(&mut self, id: &str, data: &[u8]) {
        unimplemented!("Replacing record with id: {} with data: {:#?}", id, data.len());
    }

    pub fn query_records(&self, query: &str) -> Result<Vec<(&MimeType, &uuid::Uuid)>, VennbaseError> {
        let parsed_query = parse_query(query)
            .map_err(|_| VennbaseError("Invalid query".into()))?;
        // FIXME: this need to be optimized. maybe using the Shunting yard algorithm?
        // https://en.wikipedia.org/wiki/Shunting_yard_algorithm
        let mut matched_records = Vec::<(&MimeType, &uuid::Uuid)>::with_capacity(4); // lucky number

        fn evaluate(db: &Vennbase, node: &ASTNode, mime: &MimeType, id: &uuid::Uuid) -> Result<bool, ()> {
            match node {
                ASTNode::Not { operand } => {
                    Ok(!evaluate(db, operand, mime, id)?)
                },
                ASTNode::And { left, right } => {
                    Ok(evaluate(db, left, mime, id)? && evaluate(db, right, mime, id)?)
                },
                ASTNode::Or { left, right } => {
                    Ok(evaluate(db, left, mime, id)? || evaluate(db, right, mime, id)?)
                },
                ASTNode::Implies { left, right } => {
                    Ok(!evaluate(db, left, mime, id)? || evaluate(db, right, mime, id)?)
                },
                ASTNode::IfAndOnlyIf { left, right } => {
                    Ok(evaluate(db, left, mime, id)? == evaluate(db, right, mime, id)?)
                },
                ASTNode::Literal { value } => {
                    Ok(*value)
                },
                ASTNode::Identifier { name: expression } => {
                    let colon_i = expression.find(':').ok_or(())?;
                    if colon_i == 0 || colon_i == expression.len() - 1 {
                        // FIXME: improve error granularity
                        // is this check really needed? (expression.rfind(':').unwrap() != colon_i)
                        return Err(());
                    }
                    // Due to the checks, `filter` and `name` must be valid strings at this point
                    let (filter_name, filter) = expression.split_at(colon_i + 1);

                    let result = match filter_name {
                        "mime:" => {
                            filter == "*" || filter == mime.as_str()
                        },
                        "id:" => {
                            filter == "*" || filter == id.to_string()
                        },
                        "tag:" => {
                            filter == "*" || db.tags.map.get(filter).map_or(false, |records| {
                                records.contains(&id.to_string())
                            })
                        },
                        _ => {
                            return Err(());
                        }
                    };
                    Ok(result)
                },
            }
        }

        // let variables = parsed_query.get_identifiers();
        // if !variables.iter().any(|v| v.contains("mime:")) {
            // Evaluate each record on every partition. MimeType doesnt matter
        for (mimetype, partition) in &self.partitions {
            for (uuid, _) in partition.iter_active_records() {
                let matches = evaluate(self, &parsed_query, mimetype, uuid)
                    .map_err(|_| VennbaseError("Failed to evaluate".into()))?;
                if matches {
                    matched_records.push((mimetype, uuid));
                }
            }
        }
        Ok(matched_records)
        // }

        // At this point, since the query contains some MimeType criteria, we have the
        // advantage of determining which partitions to skip
        // NOTE: this has no advantage for small partitions.
        // for (mimetype, partition) in &self.partitions {
        //     let should_evaluate = false;
        //     // Decide if we should skip the partition based on its mimetype

        //     // in order to permutate this correctly, we should have:
        //     // HashMap<String, VariablePermutation>
        //     let variables_table = fix_boolean_values_for_mimetype(variables, mimetype);
        //     // variable table should make mimetypes fickle!!!

        //     // if some permutation of the fickle permutation matches, then the partition
        //     // should be evaluated.
        //     for permutation in permutate_fickle(variables_table) {
        //         // evaluate this permutation
        //         if should_evaluate = evaluate_for_table(&parsed_query, &permutation).unwrap() {
        //             break;
        //         }
        //     }

        //     if should_evaluate {
        //         for record in partition.iter_active_records() {
        //             if evaluate_for_record(&parsed_query, &record, &mimetype) == true {
        //                 matched_records.push(record);
        //             }
        //         }
        //     }
        // }

    }

    pub fn fetch_record_by_id(
        &self,
        record_id: &uuid::Uuid,
        resize_dims: &Option<Dimensions>
    ) -> io::Result<Option<(&MimeType, StoredRecord)>> {
        for (mimetype, partition) in &self.partitions {
            if let Some(mut record) = partition.fetch_record(record_id)? {
                let size = record.limit();
                // If we need to resize the image
                if is_resizable_format(mimetype) && resize_dims.is_some() {
                    let new_dimensions = resize_dims.as_ref().unwrap();
                    // Load the entire image into memory
                    let mut data = Vec::with_capacity(size as usize);
                    record.read_to_end(&mut data)?;
                    let resize_result = resize_image(
                        &data,
                        // MIMEtype should be valid at this point
                        ImageFormat::from_mime_type(mimetype.as_str()).unwrap(),
                        new_dimensions
                    );
                    let data = match resize_result {
                        Ok(data) => data,
                        Err(_) => {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "Failed to resize image"
                            ));
                        },
                    };

                    return Ok(
                        Some((mimetype, StoredRecord::InMemoryRecord(data)))
                    );
                }
                // Otherwise, send the image as it is
                return Ok(
                    Some((mimetype, StoredRecord::InDiskRecord(record)))
                );
            }
        }
        Ok(None)
    }

    pub fn get_tags_for_record(&self, record_id: &uuid::Uuid) -> Vec<&str> {
        self.tags.get_tags_for_id(record_id)
    }

    fn parse_dir_tree(path: &str) -> io::Result<Vennbase> {
        let dir = fs::read_dir(path)?;
        let mut partitions: HashMap<MimeType, Partition> = HashMap::new();

        for entry in dir {
            // Read the filename
            let filepath = entry?.path();
            if filepath.is_dir() || filepath.file_name().unwrap() == ".map" {
                continue;
            }
            let mimetype = MimeType::from_base64_filename(filepath.file_name().unwrap())?;
            println!("Found partition: {:#?}", mimetype);

            partitions.insert(
                mimetype,
                Partition::from_file(filepath)?
            );
        }

        let mut tags_map = {
            let map_path = PathBuf::from(path).join(".map");
            let file = File::open(map_path)?;
            let reader = BufReader::new(file);
            serde_json::from_reader::<_, InvertedIndexMap>(reader)?
        };

        tags_map.path = PathBuf::from(path).join(".map");

        Ok(Vennbase {
            path: path.into(),
            partitions,
            tags: tags_map,
        })
    }

    /// Creates a new partition for the database with the given Mime Type.
    ///
    /// Caller should ensure that the partition does not exist yet, or the whole file will be
    /// truncated.
    fn create_new_partition(&mut self, mimetype: MimeType) -> io::Result<&mut Partition> {
        let partition_path = self.path.join(mimetype.to_base64_pathname());
        assert!(!partition_path.exists());
        println!("New partition: {partition_path:?}");
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
