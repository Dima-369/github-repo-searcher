use clap::{Arg, Command};
use octocrab::Octocrab;
use skim::prelude::*;
use std::process::Command as StdCommand;
use std::error::Error;
use std::sync::Arc;
use std::borrow::Cow;
use std::thread;
use std::time::Duration;
extern crate libc;

pub mod filter;

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
    let octocrab = Octocrab::builder().personal_token(token.to_string()).build()?;
    let mut page = octocrab
        .current()
        .list_repos_for_authenticated_user()
        .per_page(100) // Maximum allowed per page
        .send()
        .await?;

    let mut all_repos = Vec::new();

    // Add repos from the first page
    all_repos.extend(
        page.items
            .into_iter()
            .map(|repo| (repo.name, repo.ssh_url.unwrap_or_default()))
    );

    // Fetch all remaining pages
    while let Some(next_page) = octocrab.get_page(&page.next).await? {
        page = next_page;
        all_repos.extend(
            page.items
                .into_iter()
                .map(|repo| (repo.name, repo.ssh_url.unwrap_or_default()))
        );
    }

    println!("Fetched {} repositories", all_repos.len());
    Ok(all_repos)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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
                .required(true),
        )
        .get_matches();

    let token = matches.get_one::<String>("token").unwrap();

    // Get all repositories
    let repos = fetch_repos(token).await?;

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
        .prompt(Some("Search (use -term to exclude): "))
        .exact(true)
        .tiebreak(Some("score".to_string()))
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

        // Extract strings from SkimItems
        let item_strings: Vec<(usize, String)> = items
            .iter()
            .enumerate()
            .map(|(i, item)| (i, item.text().to_string()))
            .collect();

        // Apply our custom filter
        let filtered_indices = filter::filter_human(&item_strings, query, |(_, s)| s.clone())
            .into_iter()
            .map(|(i, _)| i)
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
    let output = Skim::run_with(&options, Some(filtered_rx))
        .map(|out| out.selected_items)
        .unwrap_or_default();

    // Get the selected item
    if output.is_empty() {
        println!("No selection made");
        // Force immediate exit
        unsafe {
            libc::_exit(0);
        }
    }

    let selection = output[0].output().to_string();

    // Extract SSH URL from selection
    if let Some((_, url)) = selection.rsplit_once(' ') {
        let url = url.trim_matches(|c| c == '(' || c == ')');

        // Copy to clipboard - use a more direct approach
        if cfg!(target_os = "macos") {
            // Use a synchronous approach to ensure copying completes
            let output = StdCommand::new("pbcopy")
                .arg(url)
                .output();

            match output {
                Ok(_) => println!("Copied SSH URL to clipboard: {}", url),
                Err(e) => eprintln!("Failed to copy to clipboard: {}", e)
            }
        } else if cfg!(target_os = "linux") {
            // Use a synchronous approach to ensure copying completes
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

        // Small delay to ensure clipboard operation completes
        thread::sleep(Duration::from_millis(100));

        // Force immediate exit
        unsafe {
            libc::_exit(0);
        }
    }

    Ok(())
}
