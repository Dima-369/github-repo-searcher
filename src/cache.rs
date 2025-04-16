use crate::github::Repository as GitHubRepo;
use crate::gitlab::Repository as GitLabRepo;
use crate::formatter::RepoSource;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime};

const CACHE_FILE: &str = ".repo-cache.json";
const CACHE_EXPIRY: Duration = Duration::from_secs(30 * 60); // 30 minutes

#[derive(Serialize, Deserialize)]
pub struct SourceCache {
    pub timestamp: u64,
    pub username: String,
}

#[derive(Serialize, Deserialize)]
pub struct CacheData {
    pub github: Option<SourceData>,
    pub gitlab: Option<SourceData>,
}

#[derive(Serialize, Deserialize)]
pub struct SourceData {
    pub cache_info: SourceCache,
    pub repositories: Vec<RepoData>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RepoData {
    pub name: String,
    pub url: String,
    pub description: String,
    pub owner: String,
    pub is_fork: bool,
    pub is_private: bool,
    pub source: RepoSource,
}

impl SourceCache {
    pub fn new(username: String) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            timestamp: now,
            username,
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

impl CacheData {
    pub fn new() -> Self {
        Self {
            github: None,
            gitlab: None,
        }
    }

    pub fn is_expired(&self) -> bool {
        // If any source cache is expired, consider the entire cache expired
        if let Some(github) = &self.github {
            if github.cache_info.is_expired() {
                return true;
            }
        }

        if let Some(gitlab) = &self.gitlab {
            if gitlab.cache_info.is_expired() {
                return true;
            }
        }

        // If no sources are present, consider it expired
        self.github.is_none() && self.gitlab.is_none()
    }

    pub fn update_github(&mut self, username: String, repositories: Vec<RepoData>) {
        self.github = Some(SourceData {
            cache_info: SourceCache::new(username),
            repositories,
        });
    }

    pub fn update_gitlab(&mut self, username: String, repositories: Vec<RepoData>) {
        self.gitlab = Some(SourceData {
            cache_info: SourceCache::new(username),
            repositories,
        });
    }

    pub fn get_all_repositories(&self) -> Vec<RepoData> {
        let mut all_repos = Vec::new();

        if let Some(github) = &self.github {
            all_repos.extend(github.repositories.clone());
        }

        if let Some(gitlab) = &self.gitlab {
            all_repos.extend(gitlab.repositories.clone());
        }

        all_repos
    }
}

// Convert GitHub repository format to our unified RepoData format
pub fn github_repo_to_repo_data(repo: &GitHubRepo) -> RepoData {
    let (name, url, description, owner, is_fork, is_private) = repo.clone();
    RepoData {
        name,
        url,
        description,
        owner,
        is_fork,
        is_private,
        source: RepoSource::GitHub,
    }
}

// Convert GitLab repository format to our unified RepoData format
pub fn gitlab_repo_to_repo_data(repo: &GitLabRepo) -> RepoData {
    let (name, url, description, owner, is_fork, is_private) = repo.clone();
    RepoData {
        name,
        url,
        description,
        owner,
        is_fork,
        is_private,
        source: RepoSource::GitLab,
    }
}

pub fn save_cache(cache_data: &CacheData) -> io::Result<()> {
    let json = serde_json::to_string_pretty(cache_data)?;
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
            Err(e) => {
                eprintln!("Error parsing cache file: {}", e);
                None
            },
        },
        Err(e) => {
            eprintln!("Error reading cache file: {}", e);
            None
        },
    }
}
