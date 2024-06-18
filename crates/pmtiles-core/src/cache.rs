use fxhash::FxHashMap as HashMap;
use std::sync::Mutex;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("failed to set key: {0}")]
    SetError(String),
}

pub trait Cache {
    fn get(&self, key: &str) -> Option<Vec<u8>>;
    fn set(&self, key: &str, data: &[u8]) -> Result<(), CacheError>;
}

pub struct InMemoryCache {
    cache: Mutex<HashMap<String, Vec<u8>>>,
}

impl InMemoryCache {
    pub fn new() -> Self {
        InMemoryCache {
            cache: Mutex::new(HashMap::default()),
        }
    }
}

impl Cache for InMemoryCache {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        match &self.cache.lock() {
            Ok(cache) => cache.get(key).cloned(),
            Err(_) => None,
        }
    }

    fn set(&self, key: &str, data: &[u8]) -> Result<(), CacheError> {
        let mut cache = self.cache.lock().unwrap();
        cache.insert(key.into(), data.into());
        Ok(())
    }
}
