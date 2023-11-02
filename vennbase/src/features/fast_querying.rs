use std::collections::{HashMap, hash_map};
use std::path::PathBuf;
use std::fs::File;
use std::io;
use serde::{Deserialize, Serialize};
use std::io::{prelude::*, BufWriter};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct InvertedIndexMap {
    #[serde(skip)]
    pub path: PathBuf,
    #[serde_as(as = "Vec<(_, _)>")]
    pub map: HashMap<String, Vec<String>>,
}

impl InvertedIndexMap {
    fn flush_data(&self) -> io::Result<()> {
        let file = File::create(&self.path)?;
        let mut writer = BufWriter::new(file);
        let serialized_map = serde_json::to_string(&self).unwrap();
        writer.write_all(serialized_map.as_bytes())?;
        Ok(())
    }

    pub fn add_tag(&mut self, tag: &str, record_id: uuid::Uuid) {
        let tag = tag.to_owned();
        let record_id = record_id.to_string();
        if let hash_map::Entry::Vacant(e) = self.map.entry(tag.clone()) {
            e.insert(vec![record_id]);
        }
        else {
            let records = self.map.get_mut(&tag).unwrap();
            if !records.contains(&record_id) {
                records.push(record_id);
            }
        }
        self.flush_data().unwrap(); // FIXME: handle error
    }

    pub fn remove_tag(&mut self, tag: &str, record_id: uuid::Uuid) {
        let tag = tag.to_owned();
        let record_id = record_id.to_string();
        if self.map.contains_key(&tag) {
            let records = self.map.get_mut(&tag).unwrap();
            if let Some(index) = records.iter().position(|r| r == &record_id) {
                records.remove(index);
            }
        }
        self.flush_data().unwrap(); // FIXME: handle error
    }

    pub fn get_tags_for_id(&self, record_id: &uuid::Uuid) -> Vec<&str> {
        let record_id = record_id.to_string();
        self.map
            .iter()
            .filter(|(_, records)| records.contains(&record_id))
            .map(|(tag, _)| tag.as_str())
            .collect()
    }
}
