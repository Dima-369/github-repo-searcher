use crate::browser;
use crate::cache;
use crate::cli;
use crate::formatter;
use crate::github;
use crate::gitlab;
use std::time::Duration;

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

/// Loads real repositories from cache or API
pub async fn load_real_repositories(
    args: &cli::AppArgs,
    all_repos: &mut Vec<cache::RepoData>,
    github_username: &mut String,
    gitlab_username: &mut String
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if we should use cache
    let use_cache = !args.force_download;

    if use_cache {
        // Try to load from cache first
        if let Some(cache_data) = cache::load_cache() {
            if !cache_data.is_expired() {
                println!("Using cached repositories from previous run");

                // Get all repositories from cache
                *all_repos = cache_data.get_all_repositories();

                // Set usernames from GitHub or GitLab cache
                if let Some(github) = &cache_data.github {
                    *github_username = github.cache_info.username.clone();
                }
                if let Some(gitlab) = &cache_data.gitlab {
                    *gitlab_username = gitlab.cache_info.username.clone();
                }

                println!("Loaded {} repositories from cache", all_repos.len());
                return Ok(());
            } else {
                println!("Cache expired, fetching fresh data...");
            }
        } else {
            println!("No cache found, fetching repositories...");
        }
    } else {
        println!("Force downloading repositories...");
    }

    // If we get here, we need to fetch fresh data
    fetch_and_cache_repos(args, all_repos, github_username, gitlab_username).await
}

/// Fetches repositories from all sources and caches them
pub async fn fetch_and_cache_repos(
    args: &cli::AppArgs,
    all_repos: &mut Vec<cache::RepoData>,
    github_username: &mut String,
    gitlab_username: &mut String
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a new cache
    let mut cache_data = cache::CacheData::new();
    
    // Fetch from GitHub if token is provided
    if let Some(github_token) = &args.github_token {
        let (gh_username, gh_repos) = github::fetch_repos(github_token).await?;
        *github_username = gh_username.clone();
        
        // Convert GitHub repos to RepoData
        let github_repo_data: Vec<cache::RepoData> = gh_repos
            .iter()
            .map(|repo| cache::github_repo_to_repo_data(repo))
            .collect();
        
        // Add to all_repos
        all_repos.extend(github_repo_data.clone());
        
        // Update cache
        cache_data.update_github(github_username.clone(), github_repo_data);
    }
    
    // Fetch from GitLab if token is provided
    if let Some(gitlab_token) = &args.gitlab_token {
        let (gl_username, gl_repos) = gitlab::fetch_repos(gitlab_token).await?;
        *gitlab_username = gl_username.clone();
        
        // Convert GitLab repos to RepoData
        let gitlab_repo_data: Vec<cache::RepoData> = gl_repos
            .iter()
            .map(|repo| cache::gitlab_repo_to_repo_data(repo))
            .collect();
        
        // Add to all_repos
        all_repos.extend(gitlab_repo_data.clone());
        
        // Update cache
        cache_data.update_gitlab(gitlab_username.clone(), gitlab_repo_data);
    }
    
    // Save the cache
    if let Err(e) = cache::save_cache(&cache_data) {
        eprintln!("Warning: Failed to save cache: {}", e);
    } else {
        println!("Cache updated successfully");
    }
    
    Ok(())
}
