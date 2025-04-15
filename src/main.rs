use clap::{Arg, Command};
use octocrab::Octocrab;
use std::process::Command as StdCommand;
use std::error::Error;
use std::thread;
use std::time::Duration;
use std::io::{self, Write, stdout, stdin};
extern crate libc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
extern crate termion;
use termion::input::TermRead;
use termion::event::Key;
use termion::raw::IntoRawMode;
use termion::screen::IntoAlternateScreen;
use termion::cursor;
use termion::color;
use termion::clear;
use termion::style;

mod filter;

// Custom UI for displaying and filtering repositories
struct FuzzyFinder {
    items: Vec<String>,
    filtered_items: Vec<String>,
    query: String,
    cursor_pos: usize,
    selected_index: usize,
    max_display: usize,
    scroll_offset: usize,
}

impl FuzzyFinder {
    fn new(items: Vec<String>) -> Self {
        let filtered_items = items.clone();
        let max_display = 10; // Number of items to display at once

        Self {
            items,
            filtered_items,
            query: String::new(),
            cursor_pos: 0,
            selected_index: 0,
            max_display,
            scroll_offset: 0,
        }
    }

    fn update_filter(&mut self) {
        // Use the filter_human function to filter items based on query
        self.filtered_items = filter::filter_human(&self.items, &self.query, |s| s.clone());

        // Reset selection if it's out of bounds
        if self.selected_index >= self.filtered_items.len() {
            self.selected_index = if self.filtered_items.is_empty() { 0 } else { self.filtered_items.len() - 1 };
        }

        // Reset scroll offset if needed
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + self.max_display {
            self.scroll_offset = self.selected_index - self.max_display + 1;
        }
    }

    fn move_cursor_up(&mut self) {
        if !self.filtered_items.is_empty() {
            if self.selected_index > 0 {
                self.selected_index -= 1;

                // Adjust scroll offset if needed
                if self.selected_index < self.scroll_offset {
                    self.scroll_offset = self.selected_index;
                }
            }
        }
    }

    fn move_cursor_down(&mut self) {
        if !self.filtered_items.is_empty() {
            if self.selected_index < self.filtered_items.len() - 1 {
                self.selected_index += 1;

                // Adjust scroll offset if needed
                if self.selected_index >= self.scroll_offset + self.max_display {
                    self.scroll_offset = self.selected_index - self.max_display + 1;
                }
            }
        }
    }

    fn render<W: Write>(&self, screen: &mut W) -> io::Result<()> {
        // Clear screen
        write!(screen, "{}{}", clear::All, cursor::Goto(1, 1))?;

        // Display header
        write!(screen, "{}{}> {}{}", color::Fg(color::Blue), style::Bold, self.query, style::Reset)?;
        write!(screen, "{}\r\n", cursor::Goto(self.cursor_pos as u16 + 3, 1))?;

        // Display items
        let display_count = std::cmp::min(self.max_display, self.filtered_items.len());
        let end_idx = std::cmp::min(self.scroll_offset + display_count, self.filtered_items.len());

        for i in self.scroll_offset..end_idx {
            let item = &self.filtered_items[i];

            // Highlight selected item
            if i == self.selected_index {
                write!(screen, "{}{}{} {}{}", color::Fg(color::Green), style::Bold, ">", item, style::Reset)?;
            } else {
                write!(screen, "  {}", item)?;
            }

            write!(screen, "\r\n")?;
        }

        // Display status line
        write!(screen, "{}{}{}\r\n", color::Fg(color::Blue), "-".repeat(50), style::Reset)?;
        write!(screen, "{}[{}/{}] Press Ctrl+C to quit, Enter to select{}",
               color::Fg(color::Yellow),
               self.filtered_items.len(),
               self.items.len(),
               style::Reset)?;

        screen.flush()?;
        Ok(())
    }

    fn run(&mut self) -> Option<String> {
        // Set up terminal
        let mut screen = stdout().into_raw_mode().unwrap()
            .into_alternate_screen().unwrap();

        // Initial render
        self.render(&mut screen).unwrap();

        // Process input
        let stdin = stdin();
        let mut keys = stdin.keys();

        loop {
            // Check if interrupted
            if INTERRUPTED.load(Ordering::SeqCst) {
                return None;
            }

            // Process key input
            if let Some(Ok(key)) = keys.next() {
                match key {
                    Key::Char('\n') | Key::Char('\r') => {
                        // Return selected item
                        if !self.filtered_items.is_empty() {
                            return Some(self.filtered_items[self.selected_index].clone());
                        }
                    },
                    Key::Char(c) => {
                        // Add character to query
                        self.query.push(c);
                        self.cursor_pos += 1;
                        self.update_filter();
                    },
                    Key::Backspace => {
                        // Remove character from query
                        if !self.query.is_empty() && self.cursor_pos > 0 {
                            self.query.pop();
                            self.cursor_pos -= 1;
                            self.update_filter();
                        }
                    },
                    Key::Up => {
                        self.move_cursor_up();
                    },
                    Key::Down => {
                        self.move_cursor_down();
                    },
                    Key::Ctrl('c') => {
                        return None;
                    },
                    _ => {}
                }

                // Re-render after each key press
                self.render(&mut screen).unwrap();
            }

            // Small sleep to prevent CPU hogging
            thread::sleep(Duration::from_millis(10));
        }
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

    print!("{}\u{2713}", "\r".repeat(50)); // Clear line and show checkmark
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
        print!("{}\u{2713}", "\r".repeat(50)); // Clear line and show checkmark
        print!("\rFetched page {} ({} repos so far)... ", page_count, all_repos.len());
        std::io::stdout().flush().unwrap();
    }

    println!("{}\u{2713}", "\r".repeat(50)); // Clear line and show checkmark
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

    // Create formatted choices for the fuzzy finder
    let choices: Vec<String> = repos
        .into_iter()
        .map(|(name, url)| format!("{} ({})", name, url))
        .collect();

    // Create and run the fuzzy finder
    let mut finder = FuzzyFinder::new(choices);
    let selection = match finder.run() {
        Some(selected) => selected,
        None => {
            println!("No selection made");
            unsafe { libc::_exit(0); }
        }
    };

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
                        println!("\n\033[1;32m\u{2713}\033[0m Copying git clone command...");
                        choice = String::from("1");
                        selected = true;
                    },
                    termion::event::Key::Char('s') => {
                        println!("\n\033[1;32m\u{2713}\033[0m Copying SSH URL...");
                        choice = String::from("2");
                        selected = true;
                    },
                    termion::event::Key::Char('o') if browser_url.is_some() => {
                        println!("\n\033[1;32m\u{2713}\033[0m Opening in browser...");
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
    }

    Ok(())
}
