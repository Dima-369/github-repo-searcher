use skim::prelude::*;
use std::borrow::Cow;
use std::process::Command as StdCommand;
use std::sync::Arc;

// Import the filter module
mod filter {
    /// Filter list by query case insensitively.
    pub fn filter_human<T, F>(items: &[T], query: &str, mapper: F) -> Vec<T>
    where
        T: Clone,
        F: Fn(&T) -> String,
    {
        if items.is_empty() {
            return Vec::new();
        }

        let trimmed = query.trim();
        if trimmed.is_empty() {
            return items.to_vec();
        }

        let mut result = Vec::new();
        let query_parts: Vec<String> = trimmed
            .to_lowercase()
            .split(' ')
            .filter(|part| !part.is_empty())
            .map(|part| part.to_string())
            .collect();

        // Sort query parts to handle exclusions first
        let query_parts = {
            let mut parts = query_parts;
            parts.sort_by(|a, b| {
                if a.starts_with('-') && !b.starts_with('-') {
                    std::cmp::Ordering::Less
                } else if !a.starts_with('-') && b.starts_with('-') {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            });
            parts
        };

        for item in items {
            let mapped = mapper(item).to_lowercase();
            let mut pass = true;

            for query_part in &query_parts {
                // Check length, so a single minus is still matched
                if query_part.len() >= 2 && query_part.starts_with('-') {
                    if mapped.contains(&query_part[1..]) {
                        pass = false;
                        break;
                    }
                } else if !mapped.contains(query_part) {
                    pass = false;
                    break;
                }
            }

            if pass {
                result.push(item.clone());
            }
        }

        result
    }
}

// Custom SkimItem implementation
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

fn main() {
    // Sample data
    let repos = vec![
        "project-alpha (git@github.com:user/project-alpha.git)",
        "awesome-app (git@github.com:user/awesome-app.git)",
        "documentation (git@github.com:user/documentation.git)",
        "api-service (git@github.com:user/api-service.git)",
        "web-frontend (git@github.com:user/web-frontend.git)",
        "mobile-app (git@github.com:user/mobile-app.git)",
        "database-tools (git@github.com:user/database-tools.git)",
        "testing-framework (git@github.com:user/testing-framework.git)",
    ];

    // Set up skim options
    let options = SkimOptionsBuilder::default()
        .height(Some("50%"))
        .multi(false)
        .preview(Some("echo {}"))
        .prompt(Some("Search (use -term to exclude): "))
        .exact(true)
        .tiebreak(Some("score".to_string()))
        .build()
        .unwrap();

    // Create a channel for sending items to skim
    let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();

    // Send all choices to skim
    for repo in repos {
        let item = CustomItem {
            text: repo.to_string(),
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
        return;
    }

    let selection = output[0].output().to_string();

    // Extract SSH URL from selection
    if let Some((_, url)) = selection.rsplit_once(' ') {
        let url = url.trim_matches(|c| c == '(' || c == ')');
        println!("Selected URL: {}", url);

        // Copy to clipboard
        if cfg!(target_os = "macos") {
            let status = StdCommand::new("pbcopy")
                .arg(url)
                .spawn()
                .and_then(|mut child| child.wait());

            if let Err(e) = status {
                eprintln!("Failed to copy to clipboard: {}", e);
            } else {
                println!("Copied SSH URL to clipboard");
            }
        } else if cfg!(target_os = "linux") {
            let status = StdCommand::new("xclip")
                .arg("-selection")
                .arg("clipboard")
                .arg(url)
                .spawn()
                .and_then(|mut child| child.wait());

            if let Err(e) = status {
                eprintln!("Failed to copy to clipboard: {}", e);
            } else {
                println!("Copied SSH URL to clipboard");
            }
        }
    }
}
