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
        .map(|(name, _url, description, _owner, is_fork, is_private)| {
            formatter::format_repository(&name, &description, is_fork, is_private)
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
        if let Some((repo_name, _url, browser_url)) = github::extract_repo_info(&selection, &username) {
            // Always open in browser
            if let Some(browser_url) = browser_url {
                println!("\nOpening repository in browser: {}", browser_url);
                println!("Username: {}", username);
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
