use octocrab::Octocrab;
use std::io::Write;

pub type Repository = (String, String, String, String, bool, bool); // (name, ssh_url, description, owner, is_fork, is_private)

pub async fn fetch_repos(token: &str) -> octocrab::Result<(String, Vec<Repository>)> {
    print!("Fetching user information... ");
    std::io::stdout().flush().unwrap();

    let octocrab = Octocrab::builder().personal_token(token.to_string()).build()?;

    // Get authenticated user information
    let user = octocrab.current().user().await?;
    let username = user.login;

    println!("✓"); // Show checkmark on its own line
    print!("Fetching repositories for {}... ", username);
    std::io::stdout().flush().unwrap();

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
            .map(|repo| (
                repo.name,
                repo.ssh_url.unwrap_or_default(),
                repo.description.unwrap_or_default(),
                username.clone(),
                repo.fork.unwrap_or(false),
                repo.private.unwrap_or(false)
            ))
    );

    print!("\r                                                  "); // Clear the line
    print!("\rFetched page {} ({} repos so far)... ", page_count, all_repos.len());
    std::io::stdout().flush().unwrap();

    // Fetch all remaining pages
    while let Some(next_page) = octocrab.get_page(&page.next).await? {
        // Add a small sleep to allow Ctrl+C to be processed
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        page_count += 1;
        page = next_page;

        all_repos.extend(
            page.items
                .into_iter()
                .map(|repo| (
                    repo.name,
                    repo.ssh_url.unwrap_or_default(),
                    repo.description.unwrap_or_default(),
                    username.clone(),
                    repo.fork.unwrap_or(false),
                    repo.private.unwrap_or(false)
                ))
        );
        print!("\r                                                  "); // Clear the line
        print!("\rFetched page {} ({} repos so far)... ", page_count, all_repos.len());
        std::io::stdout().flush().unwrap();
    }

    println!("✓"); // Show checkmark on its own line
    println!("Fetched {} repositories from {} pages", all_repos.len(), page_count);
    Ok((username, all_repos))
}

pub fn generate_dummy_repos() -> (String, Vec<Repository>) {
    println!("Using 100 dummy repositories for testing");
    let username = "dima-369".to_string();

    // Generate 100 dummy repositories with different names and categories
    let mut dummy_repos = Vec::with_capacity(100);

    // Add some special repositories that are easy to find
    dummy_repos.push(("clj-basic-image-cache-server".to_string(), "git@github.com:dima-369/clj-basic-image-cache-server.git".to_string(), "A basic image cache server written in Clojure".to_string(), username.clone(), true, false));
    dummy_repos.push(("rust-web-server".to_string(), "git@github.com:dima-369/rust-web-server.git".to_string(), "A web server written in Rust".to_string(), username.clone(), false, true));
    dummy_repos.push(("go-microservices".to_string(), "git@github.com:dima-369/go-microservices.git".to_string(), "Microservices examples in Go".to_string(), username.clone(), false, false));

    // Add repositories by category
    let categories = ["api", "web", "mobile", "backend", "frontend", "database", "utils", "tools", "docs", "test"];

    for i in 1..=97 {
        let category = categories[i % categories.len()];
        let name = format!("{}-project-{}", category, i);
        let url = format!("git@github.com:{}/{}.git", username, name);
        let description = format!("A {} project for {}", category, if i % 2 == 0 { "development" } else { "production" });
        // Make some repos forks and some private for variety
        let is_fork = i % 5 == 0;  // Every 5th repo is a fork
        let is_private = i % 7 == 0; // Every 7th repo is private
        dummy_repos.push((name, url, description, username.clone(), is_fork, is_private));
    }

    (username, dummy_repos)
}

pub fn extract_repo_info(selection: &str, username: &str) -> Option<(String, String, Option<String>)> {
    // Extract repository name and description from selection
    let repo_name = if let Some((name, _description_part)) = selection.split_once(" (") {
        // Selection has a description in parentheses
        name
    } else {
        // Selection is just the repo name without description
        selection
    };

    // Construct a URL based on the repository name and username
    let url = format!("git@github.com:{}/{}.git", username, repo_name);

    // Extract GitHub repo path for browser URL
    let browser_url = Some(format!("https://github.com/{}/{}", username, repo_name));

    Some((repo_name.to_string(), url, browser_url))
}
