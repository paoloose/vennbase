use std::{collections::HashMap, hash::Hash, rc::Rc};

struct LRUCacheList<V> {
    size: i32,
    prev: Option<Box<LRUCacheList<V>>>,
    next: Option<Box<LRUCacheList<V>>>,
    val: Option<Rc<V>>,
}

impl<V> LRUCacheList<V> {
    fn new() -> LRUCacheList<V> {
        LRUCacheList {
            size: 0,
            prev: None,
            next: None,
            val: None,
        }
    }
}

/// A Least Recently Used (LRU) cache
pub struct LRUCache<K, V> {
    ttl: u32,
    max_size: u32,
    records_map: HashMap<K, V>,
    records_list: LRUCacheList<V>,
    calc_size: fn(val: &V) -> i32
}

impl<K: Eq + Hash, V> LRUCache<K, V> {
    pub fn get(&self, key: K) -> Option<&V> {
        match self.records_map.get(&key) {
            Some(record) => Some(record),
            None => todo!(),
        }
    }

    pub fn save(&mut self, _key: K, val: V) {
        let _item_size = (self.calc_size)(&val);
        let _val_ptr = Rc::new(val);
    }
}

pub struct LRUCacheBuilder<V> {
    ttl: u32,
    max_size: u32,
    calc_size: fn(val: &V) -> i32
}

impl<V> LRUCacheBuilder<V> {
    pub fn new() -> Self {
        LRUCacheBuilder::<V> {
            ttl: 0,
            max_size: 0,
            calc_size: |_| 1
        }
    }

    pub fn with_ttl(mut self, ttl: u32) -> Self {
        self.ttl = ttl;
        self
    }

    pub fn with_max_size(mut self, max_size: u32) -> Self {
        self.max_size = max_size;
        self
    }

    pub fn with_calc_size(mut self, calc_size: fn(val: &V) -> i32) -> Self {
        self.calc_size = calc_size;
        self
    }

    pub fn build(self) -> LRUCache<String, V> {
        LRUCache::<String, V> {
            ttl: self.ttl,
            max_size: self.max_size,
            calc_size: self.calc_size,
            records_list: LRUCacheList::new(),
            records_map: HashMap::new()
        }
    }
}

impl<V> Default for LRUCacheBuilder<V> {
    fn default() -> Self {
        Self::new()
    }
}
