use std::error::Error;
use std::process;
use std::time::Duration;

mod cache;
mod cli;
mod filter;
mod fuzzy_finder;
mod github;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse command line arguments
    let args = cli::parse_args();

    // Get repositories (either real or dummy) and the username
    let (username, repos) = if args.use_dummy {
        github::generate_dummy_repos()
    } else {
        let token = args.token.as_ref().unwrap();

        // Check if we have a valid cache
        let use_cache = !args.force_download;
        if use_cache {
            if let Some(cache_data) = cache::load_cache() {
                if !cache_data.is_expired() {
                    println!("Using cached repositories from previous run");
                    (cache_data.username, cache_data.repositories)
                } else {
                    println!("Cache expired, fetching fresh data...");
                    let (username, repos) = github::fetch_repos(token).await?;
                    // Save to cache in the background
                    let _ = cache::save_cache(&username, &repos);
                    (username, repos)
                }
            } else {
                println!("No cache found, fetching repositories...");
                let (username, repos) = github::fetch_repos(token).await?;
                // Save to cache
                let _ = cache::save_cache(&username, &repos);
                (username, repos)
            }
        } else {
            println!("Force downloading repositories...");
            let (username, repos) = github::fetch_repos(token).await?;
            // Save to cache
            let _ = cache::save_cache(&username, &repos);
            (username, repos)
        }
    };

    // Create formatted choices for the fuzzy finder
    let choices: Vec<String> = repos
        .into_iter()
        .map(|(name, _url, description, _owner)| {
            if description.is_empty() {
                name.clone()
            } else {
                format!("{} ({})", name, description)
            }
        })
        .collect();

    // Create and run the fuzzy finder
    let mut finder = fuzzy_finder::FuzzyFinder::new(choices);
    let selection = match finder.run() {
        Some(selected) => selected,
        None => {
            println!("No selection made");
            process::exit(0);
        }
    };

    // Extract repository name and URL from selection
    if let Some((repo_name, _url, browser_url)) = github::extract_repo_info(&selection, &username) {
        // Always open in browser
        if let Some(browser_url) = browser_url {
            println!("\nOpening repository in browser: {}", browser_url);
            println!("Username: {}", username);
            println!("Repository: {}", repo_name);

            // Write URL to a file for debugging
            use std::fs::File;
            use std::io::Write;
            let mut file = File::create("url_debug.txt").unwrap();
            writeln!(file, "URL: {}", browser_url).unwrap();
            writeln!(file, "Username: {}", username).unwrap();
            writeln!(file, "Repository: {}", repo_name).unwrap();

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
        } else {
            println!("No browser URL available for repository: {}", repo_name);
        }
    } else {
        println!("Error: Could not parse repository information from selection");
    }

    Ok(())
}
