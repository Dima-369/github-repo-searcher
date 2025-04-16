use crate::github::Repository;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime};

const CACHE_FILE: &str = ".gh-repo-cache.json";
const CACHE_EXPIRY: Duration = Duration::from_secs(30 * 60); // 30 minutes

#[derive(Serialize, Deserialize)]
pub struct CacheData {
    pub timestamp: u64,
    pub username: String,
    pub repositories: Vec<Repository>,
}

impl CacheData {
    pub fn new(username: String, repositories: Vec<Repository>) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            timestamp: now,
            username,
            repositories,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now - self.timestamp > CACHE_EXPIRY.as_secs()
    }
}

pub fn save_cache(username: &str, repositories: &[Repository]) -> io::Result<()> {
    let cache_data = CacheData::new(username.to_string(), repositories.to_vec());
    let json = serde_json::to_string_pretty(&cache_data)?;
    fs::write(CACHE_FILE, json)?;
    Ok(())
}

pub fn load_cache() -> Option<CacheData> {
    if !Path::new(CACHE_FILE).exists() {
        return None;
    }

    match fs::read_to_string(CACHE_FILE) {
        Ok(json) => match serde_json::from_str(&json) {
            Ok(cache_data) => Some(cache_data),
            Err(_) => None,
        },
        Err(_) => None,
    }
}
