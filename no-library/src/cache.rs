use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Eq, PartialEq, Hash, Clone)]
pub struct NamespacedName {
    pub namespace: String,
    pub name: String,
}

impl NamespacedName {
    pub fn new(name: &str, namespace: &str) -> Self {
        NamespacedName {
            name: String::from(name),
            namespace: String::from(namespace),
        }
    }
}

#[derive(Clone)]
pub struct CacheEntry {
    pub resource_version: String,
    pub generation: Option<u64>,
}

impl CacheEntry {
    pub fn new(resource_version: &str, generation: u64) -> Self {
        CacheEntry {
            resource_version: String::from(resource_version),
            generation: Some(generation),
        }
    }

    pub fn new_no_generation(resource_version: &str) -> Self {
        CacheEntry {
            resource_version: String::from(resource_version),
            generation: None,
        }
    }
}

pub type Cache = Arc<Mutex<HashMap<NamespacedName, CacheEntry>>>;

pub fn new_cache() -> Cache {
    Arc::new(Mutex::new(HashMap::new()))
}

pub fn clone_cache(cache: &Cache) -> Cache {
    Arc::clone(cache)
}
