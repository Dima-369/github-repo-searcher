use std::io::{self, Write};
use crate::clipboard::{self, ClipboardContent};

pub struct MenuOption {
    pub key: char,
    pub description: String,
    pub value: String,
}

pub fn display_menu(repo_name: &str, options: &[MenuOption]) {
    // Show interactive menu with instant action
    println!("\nSelected: {}\n", repo_name);

    // Display the interactive menu
    println!("\033[1;36mInteractive Menu:\033[0m"); // Cyan bold text
    for option in options {
        println!("  \033[1;33m[{}]\033[0m \033[1m{}:\033[0m \033[90m{}\033[0m", 
                 option.key, option.description, option.value);
    }
    println!("\nPress the key for your choice (or Ctrl+C to cancel): ");
}

pub fn handle_menu_choice(
    clone_cmd: &str, 
    ssh_url: &str, 
    browser_url: &Option<String>
) -> io::Result<()> {
    // Setup terminal for single key input
    let stdin = io::stdin();
    let mut buffer = String::new();

    // Read a single character from stdin
    let choice = match stdin.read_line(&mut buffer) {
        Ok(_) => {
            let input = buffer.trim();
            if input == "c" {
                println!("\n\033[1;32m\u{2713}\033[0m Copying git clone command...");
                "1"
            } else if input == "s" {
                println!("\n\033[1;32m\u{2713}\033[0m Copying SSH URL...");
                "2"
            } else if input == "o" && browser_url.is_some() {
                println!("\n\033[1;32m\u{2713}\033[0m Opening in browser...");
                "3"
            } else {
                println!("\nInvalid choice, using default (copy git clone command)...");
                "1"
            }
        },
        Err(e) => {
            eprintln!("Error reading input: {}", e);
            println!("Using default choice (copy git clone command)...");
            "1"
        }
    };

    match choice {
        "1" => {
            // Copy git clone command
            match clipboard::copy_to_clipboard(ClipboardContent::GitCloneCommand(clone_cmd.to_string())) {
                Ok(msg) => println!("{}", msg),
                Err(e) => eprintln!("{}", e)
            }
        },
        "2" => {
            // Copy SSH URL
            match clipboard::copy_to_clipboard(ClipboardContent::SshUrl(ssh_url.to_string())) {
                Ok(msg) => println!("{}", msg),
                Err(e) => eprintln!("{}", e)
            }
        },
        "3" if browser_url.is_some() => {
            // Open in browser
            let url = browser_url.as_ref().unwrap();
            match clipboard::open_in_browser(url) {
                Ok(msg) => println!("{}", msg),
                Err(e) => eprintln!("{}", e)
            }
        },
        _ => {
            println!("Invalid choice. Exiting.");
        }
    }

    Ok(())
}
