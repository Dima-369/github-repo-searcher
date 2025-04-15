use std::io::{self, Write};

// Import the filter module directly

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

    println!("Available repositories:");
    for (i, repo) in repos.iter().enumerate() {
        println!("{}. {}", i + 1, repo);
    }

    println!("\nEnter search query (use '-term' to exclude): ");
    io::stdout().flush().unwrap();
    let mut query = String::new();
    io::stdin().read_line(&mut query).unwrap();

    // Apply custom human filter
    let filtered_choices = filter::filter_human(&repos, &query, |s| s.to_string());

    if filtered_choices.is_empty() {
        println!("No matches found for query: {}", query.trim());
        return;
    }

    // Display filtered choices with numbers
    println!("\nMatching repositories:");
    for (i, choice) in filtered_choices.iter().enumerate() {
        println!("{}: {}", i + 1, choice);
    }

    // Get user selection
    print!("\nSelect repository (enter number): ");
    io::stdout().flush().unwrap();
    let mut selection_input = String::new();
    io::stdin().read_line(&mut selection_input).unwrap();

    let selection_num = match selection_input.trim().parse::<usize>() {
        Ok(num) if num > 0 && num <= filtered_choices.len() => num - 1,
        _ => {
            println!("Invalid selection");
            return;
        }
    };

    let selection = &filtered_choices[selection_num];

    // Extract SSH URL from selection
    if let Some((_, url)) = selection.rsplit_once(' ') {
        let url = url.trim_matches(|c| c == '(' || c == ')');
        println!("Selected URL: {}", url);

        // Copy to clipboard
        if cfg!(target_os = "macos") {
            let status = std::process::Command::new("pbcopy")
                .arg(url)
                .spawn()
                .and_then(|mut child| child.wait());

            if let Err(e) = status {
                eprintln!("Failed to copy to clipboard: {}", e);
            } else {
                println!("Copied SSH URL to clipboard");
            }
        } else if cfg!(target_os = "linux") {
            let status = std::process::Command::new("xclip")
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
