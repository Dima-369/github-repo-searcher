use std::error::Error;
use std::io::Write;
use std::process;
use std::time::Duration;
extern crate termion;
use termion::input::TermRead;

// Function to clean up terminal state before exiting
fn cleanup_terminal() {
    // Ensure terminal is in a clean state
    print!("{}{}", termion::screen::ToMainScreen, termion::cursor::Show);
    std::io::stdout().flush().unwrap();

    // Reset terminal attributes to ensure proper cleanup
    if let Ok(_) = termion::get_tty() {
        let _ = termion::async_stdin().keys().next(); // Consume any pending input
        let _ = termion::terminal_size(); // Force terminal refresh
    }
}

// No platform-specific imports needed with ctrlc crate

mod cache;
mod cli;
mod filter;
mod formatter;
mod fuzzy_finder;
mod github;
mod gitlab;

// Helper function to fetch repositories from all sources and cache them
async fn fetch_and_cache_repos(
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

// Set up a Ctrl+C handler that works globally
fn setup_ctrl_c_handler() {
    // Use the ctrlc crate which works reliably across platforms
    ctrlc::set_handler(move || {
        cleanup_terminal();
        println!("\nReceived Ctrl+C, exiting...");
        process::exit(0);
    }).expect("Error setting Ctrl+C handler");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Set up global Ctrl+C handler
    setup_ctrl_c_handler();
    // Parse command line arguments
    let args = cli::parse_args();

    // Use the RepoData struct from the cache module
    use cache::RepoData;

    // Get repositories (either real or dummy) and combine them
    let mut all_repos: Vec<RepoData> = Vec::new();
    let mut github_username = String::new();
    let mut gitlab_username = String::new();

    if args.use_dummy {
        // Use dummy data
        let (dummy_username, dummy_repos) = github::generate_dummy_repos();
        github_username = dummy_username.clone();
        gitlab_username = "Gira".to_string(); // Default GitLab username for dummy data

        // Convert to RepoData with GitHub source
        all_repos.extend(dummy_repos.into_iter().map(|(name, url, description, owner, is_fork, is_private)| {
            RepoData {
                name,
                url,
                description,
                owner,
                is_fork,
                is_private,
                source: formatter::RepoSource::GitHub,
            }
        }));
    } else {
        // Fetch real repositories
        // Check if we have a valid cache
        let use_cache = !args.force_download;

        if use_cache && !args.force_download {
            // Try to load cache
            if let Some(cache_data) = cache::load_cache() {
                if !cache_data.is_expired() {
                    println!("Using cached repositories from previous run");

                    // Get all repositories from cache
                    all_repos = cache_data.get_all_repositories();

                    // Set usernames from GitHub or GitLab cache
                    if let Some(github) = &cache_data.github {
                        github_username = github.cache_info.username.clone();
                    }
                    if let Some(gitlab) = &cache_data.gitlab {
                        gitlab_username = gitlab.cache_info.username.clone();
                    }

                    println!("Loaded {} repositories from cache", all_repos.len());
                } else {
                    // Cache expired, fetch fresh data
                    println!("Cache expired, fetching fresh data...");
                    fetch_and_cache_repos(&args, &mut all_repos, &mut github_username, &mut gitlab_username).await?;
                }
            } else {
                // No cache, fetch fresh data
                println!("No cache found, fetching repositories...");
                fetch_and_cache_repos(&args, &mut all_repos, &mut github_username, &mut gitlab_username).await?;
            }
        } else {
            // Force download
            println!("Force downloading repositories...");
            fetch_and_cache_repos(&args, &mut all_repos, &mut github_username, &mut gitlab_username).await?;
        }
    }

    // Print summary of repositories found
    let github_count = all_repos.iter().filter(|r| matches!(r.source, formatter::RepoSource::GitHub)).count();
    let gitlab_count = all_repos.iter().filter(|r| matches!(r.source, formatter::RepoSource::GitLab)).count();
    println!("Found {} repositories: {} from GitHub, {} from GitLab", all_repos.len(), github_count, gitlab_count);

    // Create formatted choices for the fuzzy finder
    let choices: Vec<String> = all_repos
        .iter()
        .map(|repo| {
            formatter::format_repository(&repo.name, &repo.description, repo.is_fork, repo.is_private, repo.source)
        })
        .collect();

    // Create the fuzzy finder
    let mut finder = fuzzy_finder::FuzzyFinder::new(choices);

    // Run the fuzzy finder in a loop
    loop {
        let selection = match finder.run() {
            Some(selected) => selected,
            None => {
                cleanup_terminal();
                println!("No selection made");
                process::exit(0);
            }
        };

        // Extract repository name and URL from selection
        // Determine if this is a GitHub or GitLab repository based on the [GH] or [GL] tag
        let is_gitlab = selection.contains(" [GL]");

        let repo_info = if is_gitlab {
            gitlab::extract_repo_info(&selection, &gitlab_username)
        } else {
            github::extract_repo_info(&selection, &github_username)
        };

        if let Some((repo_name, _url, browser_url)) = repo_info {
            // Always open in browser
            if let Some(browser_url) = browser_url {
                println!("\nOpening repository in browser: {}", browser_url);
                println!("Username: {}", if is_gitlab { &gitlab_username } else { &github_username });
                println!("Repository: {}", repo_name);

                // Open URL in browser
                #[cfg(target_os = "macos")]
                {
                    process::Command::new("open")
                        .arg(&browser_url)
                        .spawn()
                        .expect("Failed to open URL in browser")
                        .wait()
                        .expect("Failed to wait on browser process");
                }

                #[cfg(target_os = "windows")]
                {
                    process::Command::new("cmd")
                        .args(["/c", "start", &browser_url])
                        .spawn()
                        .expect("Failed to open URL in browser")
                        .wait()
                        .expect("Failed to wait on browser process");
                }

                #[cfg(target_os = "linux")]
                {
                    process::Command::new("xdg-open")
                        .arg(&browser_url)
                        .spawn()
                        .expect("Failed to open URL in browser")
                        .wait()
                        .expect("Failed to wait on browser process");
                }

                // Small delay to ensure operation completes
                tokio::time::sleep(Duration::from_millis(100)).await;

                // Continue running the fuzzy finder
                println!("\nPress any key to continue searching or Ctrl+C/Esc to exit...");
                tokio::time::sleep(Duration::from_secs(1)).await;
            } else {
                println!("No browser URL available for repository: {}", repo_name);
            }
        } else {
            println!("Error: Could not parse repository information from selection");
        }
    }
    // The loop above never exits normally, only through Ctrl+C or Esc
    // which call process::exit(0), so this is unreachable
}
