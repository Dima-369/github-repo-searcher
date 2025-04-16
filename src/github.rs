use octocrab::Octocrab;
use std::io::Write;

pub type Repository = (String, String, String); // (name, ssh_url, description)

pub async fn fetch_repos(token: &str) -> octocrab::Result<Vec<Repository>> {
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
            .map(|repo| (
                repo.name,
                repo.ssh_url.unwrap_or_default(),
                repo.description.unwrap_or_default()
            ))
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
                .map(|repo| (
                    repo.name,
                    repo.ssh_url.unwrap_or_default(),
                    repo.description.unwrap_or_default()
                ))
        );
        print!("{}\u{2713}", "\r".repeat(50)); // Clear line and show checkmark
        print!("\rFetched page {} ({} repos so far)... ", page_count, all_repos.len());
        std::io::stdout().flush().unwrap();
    }

    println!("{}\u{2713}", "\r".repeat(50)); // Clear line and show checkmark
    println!("\rFetched {} repositories from {} pages", all_repos.len(), page_count);
    Ok(all_repos)
}

pub fn generate_dummy_repos() -> Vec<Repository> {
    println!("Using 100 dummy repositories for testing");

    // Generate 100 dummy repositories with different names and categories
    let mut dummy_repos = Vec::with_capacity(100);

    // Add some special repositories that are easy to find
    dummy_repos.push(("clj-basic-image-cache-server".to_string(), "git@github.com:user/clj-basic-image-cache-server.git".to_string(), "A basic image cache server written in Clojure".to_string()));
    dummy_repos.push(("rust-web-server".to_string(), "git@github.com:user/rust-web-server.git".to_string(), "A web server written in Rust".to_string()));
    dummy_repos.push(("go-microservices".to_string(), "git@github.com:user/go-microservices.git".to_string(), "Microservices examples in Go".to_string()));

    // Add repositories by category
    let categories = ["api", "web", "mobile", "backend", "frontend", "database", "utils", "tools", "docs", "test"];

    for i in 1..=97 {
        let category = categories[i % categories.len()];
        let name = format!("{}-project-{}", category, i);
        let url = format!("git@github.com:user/{}.git", name);
        let description = format!("A {} project for {}", category, if i % 2 == 0 { "development" } else { "production" });
        dummy_repos.push((name, url, description));
    }

    dummy_repos
}

pub fn extract_repo_info(selection: &str, username: &str) -> Option<(String, String, Option<String>)> {
    // Extract repository name and description from selection
    let repo_name = if let Some((name, description_part)) = selection.split_once(" (") {
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
