use crate::browser;
use crate::cache;
use crate::cli;
use crate::formatter;
use crate::github;
use crate::gitlab;
use std::time::Duration;
use tokio::sync::mpsc;

/// Processes a selected repository by extracting its information and opening it in the browser
pub async fn process_repository_selection(
    selection: &str,
    github_username: &str,
    gitlab_username: &str
) -> Result<(), Box<dyn std::error::Error>> {
    // Determine if this is a GitHub or GitLab repository based on the [GH] or [GL] tag
    let is_gitlab = selection.contains(" [GL]");

    // Extract repository information based on the source
    let repo_info = if is_gitlab {
        gitlab::extract_repo_info(selection, gitlab_username)
    } else {
        github::extract_repo_info(selection, github_username)
    };

    // Process the repository information
    if let Some((repo_name, _url, browser_url)) = repo_info {
        // Open in browser if URL is available
        if let Some(browser_url) = browser_url {
            // Display repository information
            let username = if is_gitlab { gitlab_username } else { github_username };
            println!("Repository: {}", repo_name);
            println!("Username: {}", username);

            // Open the URL in the browser
            browser::open_in_browser(&browser_url).await?;

            // Continue running the fuzzy finder
            println!("\nPress any key to continue searching or Ctrl+C/Esc to exit...");
            tokio::time::sleep(Duration::from_secs(1)).await;
        } else {
            println!("No browser URL available for repository: {}", repo_name);
        }
    } else {
        println!("Error: Could not parse repository information from selection");
    }

    Ok(())
}

/// Loads dummy repositories for testing
pub fn load_dummy_repositories(
    all_repos: &mut Vec<cache::RepoData>,
    github_username: &mut String,
    gitlab_username: &mut String
) {
    // Get dummy GitHub repositories
    let (dummy_username, dummy_repos) = github::generate_dummy_repos();
    *github_username = dummy_username.clone();
    *gitlab_username = "Gira".to_string(); // Default GitLab username for dummy data

    // Convert to RepoData with GitHub source
    all_repos.extend(dummy_repos.into_iter().map(|(name, url, description, owner, is_fork, is_private)| {
        cache::RepoData {
            name,
            url,
            description,
            owner,
            is_fork,
            is_private,
            source: formatter::RepoSource::GitHub,
        }
    }));
}

/// Message type for repository updates
pub enum RepoUpdateMessage {
    /// New repositories have been loaded
    NewRepos {
        repos: Vec<cache::RepoData>,
        github_username: String,
        gitlab_username: String,
    },
    /// Background loading has completed
    LoadingComplete,
    /// An error occurred during loading
    Error(String),
    /// Status update message
    Status(String),
}

/// Loads repositories with background refresh
pub async fn load_repositories_with_background_refresh(
    args: &cli::AppArgs,
    all_repos: &mut Vec<cache::RepoData>,
    github_username: &mut String,
    gitlab_username: &mut String,
    tx: mpsc::Sender<RepoUpdateMessage>
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if we should use cache
    let use_cache = !args.force_download;
    let mut cache_loaded = false;

    if use_cache {
        // Try to load from cache first
        if let Some(cache_data) = cache::load_cache() {
            if !cache_data.is_expired() {
                // Send status message
                let _ = tx.send(RepoUpdateMessage::Status("Using cached repositories".to_string())).await;

                // Get all repositories from cache
                *all_repos = cache_data.get_all_repositories();

                // Set usernames from GitHub or GitLab cache
                if let Some(github) = &cache_data.github {
                    *github_username = github.cache_info.username.clone();
                }
                if let Some(gitlab) = &cache_data.gitlab {
                    *gitlab_username = gitlab.cache_info.username.clone();
                }

                let _ = tx.send(RepoUpdateMessage::Status(
                    format!("Loaded {} repositories from cache", all_repos.len())
                )).await;

                cache_loaded = true;
            } else {
                let _ = tx.send(RepoUpdateMessage::Status("Cache expired, will fetch fresh data in background".to_string())).await;
            }
        } else {
            let _ = tx.send(RepoUpdateMessage::Status("No cache found, will fetch repositories in background".to_string())).await;
        }
    } else {
        let _ = tx.send(RepoUpdateMessage::Status("Force downloading repositories in background".to_string())).await;
    }

    // Clone arguments for the background task
    let github_token = args.github_token.clone();
    let gitlab_token = args.gitlab_token.clone();
    let tx_clone = tx.clone();

    // Start background task to fetch fresh data
    spawn_background_task(github_token.clone(), gitlab_token.clone(), tx_clone.clone());

    // If we didn't load from cache, we need to wait for the background task to provide initial data
    if !cache_loaded && all_repos.is_empty() {
        let _ = tx.send(RepoUpdateMessage::Status("Waiting for initial repository data...".to_string())).await;
    }

    Ok(())
}

/// Spawns a background task to fetch repositories
fn spawn_background_task(
    github_token: Option<String>,
    gitlab_token: Option<String>,
    tx: mpsc::Sender<RepoUpdateMessage>
) {
    // Use a thread instead of a task to avoid Send issues
    std::thread::spawn(move || {
        // Create a new runtime for this thread
        let rt = tokio::runtime::Runtime::new().unwrap();

        // Run the async code in the new runtime
        rt.block_on(async {
            // Create a new cache
            let mut cache_data = cache::CacheData::new();
            let mut all_repos = Vec::new();
            let mut github_username = String::new();
            let mut gitlab_username = String::new();

            // Fetch from GitHub if token is provided
            if let Some(github_token) = &github_token {
                let _ = tx.send(RepoUpdateMessage::Status("Fetching GitHub repositories...".to_string())).await;

                match github::fetch_repos(github_token).await {
                    Ok((gh_username, gh_repos)) => {
                        github_username = gh_username.clone();

                        // Convert GitHub repos to RepoData
                        let github_repo_data: Vec<cache::RepoData> = gh_repos
                            .iter()
                            .map(|repo| cache::github_repo_to_repo_data(repo))
                            .collect();

                        // Add to all_repos
                        all_repos.extend(github_repo_data.clone());

                        // Update cache
                        cache_data.update_github(github_username.clone(), github_repo_data);

                        // Send update message with the GitHub repos
                        let _ = tx.send(RepoUpdateMessage::NewRepos {
                            repos: all_repos.clone(),
                            github_username: github_username.clone(),
                            gitlab_username: gitlab_username.clone(),
                        }).await;

                        let _ = tx.send(RepoUpdateMessage::Status(
                            format!("Fetched {} GitHub repositories", gh_repos.len())
                        )).await;
                    },
                    Err(e) => {
                        // Format error message before sending to avoid Send issues
                        let error_msg = format!("GitHub error: {}", e);
                        let _ = tx.send(RepoUpdateMessage::Error(error_msg)).await;
                    }
                }
            }

            // Fetch from GitLab if token is provided
            if let Some(gitlab_token) = &gitlab_token {
                let _ = tx.send(RepoUpdateMessage::Status("Fetching GitLab repositories...".to_string())).await;

                match gitlab::fetch_repos(gitlab_token).await {
                    Ok((gl_username, gl_repos)) => {
                        gitlab_username = gl_username.clone();

                        // Convert GitLab repos to RepoData
                        let gitlab_repo_data: Vec<cache::RepoData> = gl_repos
                            .iter()
                            .map(|repo| cache::gitlab_repo_to_repo_data(repo))
                            .collect();

                        // Add to all_repos
                        all_repos.extend(gitlab_repo_data.clone());

                        // Update cache
                        cache_data.update_gitlab(gitlab_username.clone(), gitlab_repo_data);

                        // Send update message with all repos
                        let _ = tx.send(RepoUpdateMessage::NewRepos {
                            repos: all_repos.clone(),
                            github_username: github_username.clone(),
                            gitlab_username: gitlab_username.clone(),
                        }).await;

                        let _ = tx.send(RepoUpdateMessage::Status(
                            format!("Fetched {} GitLab repositories", gl_repos.len())
                        )).await;
                    },
                    Err(e) => {
                        // Format error message before sending to avoid Send issues
                        let error_msg = format!("GitLab error: {}", e);
                        let _ = tx.send(RepoUpdateMessage::Error(error_msg)).await;
                    }
                }
            }

            // Save the cache
            match cache::save_cache(&cache_data) {
                Ok(_) => {
                    let _ = tx.send(RepoUpdateMessage::Status("Cache updated successfully".to_string())).await;
                },
                Err(e) => {
                    // Format error message before sending to avoid Send issues
                    let error_msg = format!("Failed to save cache: {}", e);
                    let _ = tx.send(RepoUpdateMessage::Error(error_msg)).await;
                }
            }

            // Signal that background loading is complete
            let _ = tx.send(RepoUpdateMessage::LoadingComplete).await;
        });
    });
}
