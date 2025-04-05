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

pub type Cache = Arc<Mutex<HashMap<NamespacedName, String>>>;

pub fn new_cache() -> Cache {
    Arc::new(Mutex::new(HashMap::new()))
}

pub fn clone_cache(cache: &Cache) -> Cache {
    Arc::clone(cache)
}
