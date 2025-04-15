use std::error::Error;
use std::thread;
use std::time::Duration;
use std::io;
extern crate libc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

mod filter;
mod cli;
mod clipboard;
mod fuzzy_finder;
mod github;
mod menu;

// Global flag to track if Ctrl+C was pressed
pub static INTERRUPTED: AtomicBool = AtomicBool::new(false);
static INIT: Once = Once::new();

// Setup signal handler for Ctrl+C
fn setup_signal_handler() {
    INIT.call_once(|| {
        ctrlc::set_handler(move || {
            INTERRUPTED.store(true, Ordering::SeqCst);
            println!("\nInterrupted, exiting...");
            unsafe { libc::_exit(0); }
        }).expect("Error setting Ctrl-C handler");
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Setup signal handler for Ctrl+C
    setup_signal_handler();

    // Parse command line arguments
    let args = cli::parse_args();

    // Get repositories (either real or dummy)
    let repos = if args.use_dummy {
        github::generate_dummy_repos()
    } else {
        let token = args.token.as_ref().unwrap();
        github::fetch_repos(token).await?
    };

    // Create formatted choices for the fuzzy finder
    let choices: Vec<String> = repos
        .into_iter()
        .map(|(name, url)| format!("{} ({})", name, url))
        .collect();

    // Create and run the fuzzy finder
    let mut finder = fuzzy_finder::FuzzyFinder::new(choices);
    let selection = match finder.run() {
        Some(selected) => selected,
        None => {
            println!("No selection made");
            unsafe { libc::_exit(0); }
        }
    };

    // Extract repository name and URL from selection
    if let Some((repo_name, url, browser_url)) = github::extract_repo_info(&selection) {
        let clone_cmd = format!("git clone {}", url);

        // Create menu options
        let mut options = vec![
            menu::MenuOption {
                key: 'c',
                description: "Copy git clone command".to_string(),
                value: clone_cmd.clone(),
            },
            menu::MenuOption {
                key: 's',
                description: "Copy SSH URL".to_string(),
                value: url.clone(),
            },
        ];

        // Add browser option if available
        if let Some(ref browser_url) = browser_url {
            options.push(menu::MenuOption {
                key: 'o',
                description: "Open in browser".to_string(),
                value: browser_url.clone(),
            });
        }

        // Display the interactive menu
        menu::display_menu(&repo_name, &options);

        // Handle user's menu choice
        menu::handle_menu_choice(&clone_cmd, &url, &browser_url)?;

        // Small delay to ensure operation completes
        thread::sleep(Duration::from_millis(100));
    } else {
        println!("Error: Could not parse repository information from selection");
    }

    Ok(())
}
