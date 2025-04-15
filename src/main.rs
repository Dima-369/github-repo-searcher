use clap::{Arg, Command};
use octocrab::Octocrab;
use skim::prelude::*;
use std::process::Command as StdCommand;
use std::error::Error;
use std::sync::Arc;
use std::borrow::Cow;
use std::thread;
use std::time::Duration;
use std::io::Write;
extern crate libc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
extern crate termion;
use termion::input::TermRead;

mod filter;

// Custom SkimItem implementation that uses our filter
struct CustomItem {
    text: String,
    preview: Option<String>,
}

impl SkimItem for CustomItem {
    fn text(&self) -> Cow<str> {
        Cow::Borrowed(&self.text)
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        match &self.preview {
            Some(p) => ItemPreview::Text(p.clone()),
            None => ItemPreview::Text(self.text.clone()),
        }
    }

    fn output(&self) -> Cow<str> {
        Cow::Borrowed(&self.text)
    }
}

// We don't need a custom matcher as we'll use a transformer instead

async fn fetch_repos(token: &str) -> octocrab::Result<Vec<(String, String)>> {
    print!("Fetching repositories... ");
    std::io::stdout().flush().unwrap();

    let octocrab = Octocrab::builder().personal_token(token.to_string()).build()?;
    let mut page = octocrab
        .current()
        .list_repos_for_authenticated_user()
        .per_page(100) // Maximum allowed per page
        .send()
        .await?;

    let mut all_repos = Vec::new();
    let mut page_count = 1;

    // Add repos from the first page
    all_repos.extend(
        page.items
            .into_iter()
            .map(|repo| (repo.name, repo.ssh_url.unwrap_or_default()))
    );

    print!("{}✓", "\r".repeat(50)); // Clear line and show checkmark
    print!("\rFetched page {} ({} repos so far)... ", page_count, all_repos.len());
    std::io::stdout().flush().unwrap();

    // Fetch all remaining pages
    while let Some(next_page) = octocrab.get_page(&page.next).await? {
        page_count += 1;
        page = next_page;

        all_repos.extend(
            page.items
                .into_iter()
                .map(|repo| (repo.name, repo.ssh_url.unwrap_or_default()))
        );
        print!("{}✓", "\r".repeat(50)); // Clear line and show checkmark
        print!("\rFetched page {} ({} repos so far)... ", page_count, all_repos.len());
        std::io::stdout().flush().unwrap();
    }

    println!("{}✓", "\r".repeat(50)); // Clear line and show checkmark
    println!("\rFetched {} repositories from {} pages", all_repos.len(), page_count);
    Ok(all_repos)
}

// Global flag to track if Ctrl+C was pressed
static INTERRUPTED: AtomicBool = AtomicBool::new(false);
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

    let matches = Command::new("gh-url-picker")
        .version("0.1.0")
        .author("Your Name <you@example.com>")
        .about("Pick GitHub repos by fuzzy filtering")
        .arg(
            Arg::new("token")
                .short('t')
                .long("token")
                .value_name("GITHUB_TOKEN")
                .help("GitHub personal access token")
                .required_unless_present("dummy"),
        )
        .arg(
            Arg::new("dummy")
                .short('d')
                .long("dummy")
                .help("Use 5 dummy repositories for testing the UI")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Check if dummy mode is enabled
    let use_dummy = matches.get_flag("dummy");

    // Get repositories (either real or dummy)
    let repos = if use_dummy {
        println!("Using dummy repositories for testing");
        vec![
            ("dummy-repo-1".to_string(), "git@github.com:user/dummy-repo-1.git".to_string()),
            ("dummy-repo-2".to_string(), "git@github.com:user/dummy-repo-2.git".to_string()),
            ("awesome-project".to_string(), "git@github.com:user/awesome-project.git".to_string()),
            ("test-repository".to_string(), "git@github.com:user/test-repository.git".to_string()),
            ("sample-code".to_string(), "git@github.com:user/sample-code.git".to_string()),
        ]
    } else {
        let token = matches.get_one::<String>("token").unwrap();
        fetch_repos(token).await?
    };

    // Create fuzzy filter choices
    let choices: Vec<String> = repos
        .into_iter()
        .map(|(name, url)| format!("{} ({})", name, url))
        .collect();

    // Set up skim options
    let options = SkimOptionsBuilder::default()
        .height(Some("50%"))
        .multi(false)
        .preview(Some("echo {}"))
        .preview_window(Some("right:50%:hidden"))
        .prompt(Some("Pick GitHub repository: "))
        .exact(true)
        .tiebreak(Some("score".to_string()))
        .bind(vec!["esc:abort"])  // Allow quitting with Escape key
        .exit0(true)  // Exit immediately with status code 0 when there's no match
        .build()
        .unwrap();

    // Create a channel for sending items to skim
    let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();

    // Send all choices to skim
    for choice in choices {
        let item = CustomItem {
            text: choice,
            preview: None,
        };
        tx.send(Arc::new(item)).unwrap();
    }
    drop(tx);

    // Create a custom filter transformer
    let transformer = Box::new(|query: &str, items: Vec<Arc<dyn SkimItem>>| {
        if query.is_empty() {
            return items;
        }

        // Check if query contains exclusion terms
        let has_exclusion = query.contains(" -");

        // If no exclusion, let skim handle it
        if !has_exclusion {
            return items;
        }

        println!("Using custom exclusion filter for query: '{}'", query);

        // Extract strings from SkimItems
        let item_strings: Vec<(usize, String)> = items
            .iter()
            .enumerate()
            .map(|(i, item)| (i, item.text().to_string()))
            .collect();

        // Apply our custom filter
        let filtered_items = filter::filter_human(&item_strings, query, |(_, s)| s.clone());
        println!("Found {} matches after exclusion filtering", filtered_items.len());

        let filtered_indices = filtered_items
            .into_iter()
            .map(|i| i.0) // Extract the index from the tuple
            .collect::<Vec<_>>();

        // If no items match, return an empty vector
        if filtered_indices.is_empty() {
            return Vec::new();
        }

        // Return only the items that passed the filter
        filtered_indices.into_iter().map(|i| items[i].clone()).collect()
    });

    // Apply our custom filter to the items before running skim
    let filtered_rx = {
        let (filtered_tx, filtered_rx): (SkimItemSender, SkimItemReceiver) = unbounded();

        // Create a thread to handle filtering
        std::thread::spawn(move || {
            let mut items = Vec::new();
            while let Ok(item) = rx.recv() {
                items.push(item);
            }

            // Apply initial filtering (empty query shows all)
            for item in transformer("", items) {
                filtered_tx.send(item).unwrap();
            }
        });

        filtered_rx
    };

    // Run skim with the filtered items
    let skim_output = Skim::run_with(&options, Some(filtered_rx));

    // Check if user aborted (pressed Escape)
    if skim_output.is_none() {
        println!("Search aborted with Escape key");
        unsafe {
            libc::_exit(0);
        }
    }

    let output = skim_output.map(|out| out.selected_items).unwrap_or_default();

    // Get the selected item
    if output.is_empty() {
        println!("No selection made");
        // Force immediate exit
        unsafe {
            libc::_exit(0);
        }
    }

    let selection = output[0].output().to_string();

    // Extract repository name and URL from selection
    if let Some((repo_name, url)) = selection.rsplit_once(' ') {
        let url = url.trim_matches(|c| c == '(' || c == ')');
        let clone_cmd = format!("git clone {}", url);

        // Extract GitHub repo path for browser URL
        let browser_url = if url.contains("github.com") {
            let parts: Vec<&str> = url.split(':').collect();
            if parts.len() > 1 {
                let repo_path = parts[1].trim_end_matches(".git");
                Some(format!("https://github.com/{}", repo_path))
            } else {
                None
            }
        } else {
            None
        };

        // Show interactive menu with instant action
        println!("\nSelected: {}\n", repo_name);

        // Create a vector of options
        let mut options = vec![
            ("c", "Copy git clone command", clone_cmd.clone()),
            ("s", "Copy SSH URL", url.to_string()),
        ];

        // Add browser option if available
        if let Some(ref browser_url) = browser_url {
            options.push(("o", "Open in browser", browser_url.clone()));
        }

        // Display the interactive menu
        println!("\033[1;36mInteractive Menu:\033[0m"); // Cyan bold text
        for (key, desc, value) in &options {
            println!("  \033[1;33m[{}]\033[0m \033[1m{}:\033[0m \033[90m{}\033[0m", key, desc, value);
        }
        println!("\nPress the key for your choice (or Ctrl+C to cancel): ");

        // Setup terminal for single key input
        let stdin = termion::async_stdin();
        let mut keys = stdin.keys();

        // Wait for a key press
        let mut choice = String::from("1"); // Default choice
        let mut selected = false;

        while !selected {
            // Check if interrupted
            if INTERRUPTED.load(Ordering::SeqCst) {
                println!("\nInterrupted, exiting...");
                unsafe { libc::_exit(0); }
            }

            // Check for key press
            if let Some(Ok(key)) = keys.next() {
                match key {
                    termion::event::Key::Char('c') => {
                        println!("\n\033[1;32m✓\033[0m Copying git clone command...");
                        choice = String::from("1");
                        selected = true;
                    },
                    termion::event::Key::Char('s') => {
                        println!("\n\033[1;32m✓\033[0m Copying SSH URL...");
                        choice = String::from("2");
                        selected = true;
                    },
                    termion::event::Key::Char('o') if browser_url.is_some() => {
                        println!("\n\033[1;32m✓\033[0m Opening in browser...");
                        choice = String::from("3");
                        selected = true;
                    },
                    termion::event::Key::Ctrl('c') => {
                        println!("\nCancelled, exiting...");
                        unsafe { libc::_exit(0); }
                    },
                    _ => {
                        // Ignore other keys
                    }
                }
            }

            // Small sleep to prevent CPU hogging
            thread::sleep(Duration::from_millis(10));
        }

        match choice.as_str() {
            "1" => {
                // Copy git clone command
                if cfg!(target_os = "macos") {
                    let output = StdCommand::new("pbcopy")
                        .arg(&clone_cmd)
                        .output();
                    match output {
                        Ok(_) => println!("Copied git clone command to clipboard: {}", clone_cmd),
                        Err(e) => eprintln!("Failed to copy to clipboard: {}", e)
                    }
                } else if cfg!(target_os = "linux") {
                    let output = StdCommand::new("xclip")
                        .arg("-selection")
                        .arg("clipboard")
                        .arg(&clone_cmd)
                        .output();
                    match output {
                        Ok(_) => println!("Copied git clone command to clipboard: {}", clone_cmd),
                        Err(e) => eprintln!("Failed to copy to clipboard: {}", e)
                    }
                } else {
                    println!("Clipboard not supported on this platform. Command: {}", clone_cmd);
                }
            },
            "2" => {
                // Copy SSH URL
                if cfg!(target_os = "macos") {
                    let output = StdCommand::new("pbcopy")
                        .arg(url)
                        .output();
                    match output {
                        Ok(_) => println!("Copied SSH URL to clipboard: {}", url),
                        Err(e) => eprintln!("Failed to copy to clipboard: {}", e)
                    }
                } else if cfg!(target_os = "linux") {
                    let output = StdCommand::new("xclip")
                        .arg("-selection")
                        .arg("clipboard")
                        .arg(url)
                        .output();
                    match output {
                        Ok(_) => println!("Copied SSH URL to clipboard: {}", url),
                        Err(e) => eprintln!("Failed to copy to clipboard: {}", e)
                    }
                } else {
                    println!("Clipboard not supported on this platform. URL: {}", url);
                }
            },
            "3" if browser_url.is_some() => {
                // Open in browser
                let browser_url = browser_url.unwrap();
                if cfg!(target_os = "macos") {
                    let output = StdCommand::new("open")
                        .arg(&browser_url)
                        .output();
                    match output {
                        Ok(_) => println!("Opened in browser: {}", browser_url),
                        Err(e) => eprintln!("Failed to open browser: {}", e)
                    }
                } else if cfg!(target_os = "linux") {
                    let output = StdCommand::new("xdg-open")
                        .arg(&browser_url)
                        .output();
                    match output {
                        Ok(_) => println!("Opened in browser: {}", browser_url),
                        Err(e) => eprintln!("Failed to open browser: {}", e)
                    }
                } else {
                    println!("Opening browser not supported on this platform. URL: {}", browser_url);
                }
            },
            _ => {
                println!("Invalid choice. Exiting.");
            }
        }

        // Small delay to ensure operation completes
        thread::sleep(Duration::from_millis(100));

        // Force immediate exit
        unsafe {
            libc::_exit(0);
        }
    }

    Ok(())
}
