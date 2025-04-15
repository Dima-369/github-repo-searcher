use octocrab::Octocrab;
use std::io::Write;

pub type Repository = (String, String); // (name, ssh_url)

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

pub fn generate_dummy_repos() -> Vec<Repository> {
    println!("Using 100 dummy repositories for testing");
    
    // Generate 100 dummy repositories with different names and categories
    let mut dummy_repos = Vec::with_capacity(100);
    
    // Add some special repositories that are easy to find
    dummy_repos.push(("awesome-project".to_string(), "git@github.com:user/awesome-project.git".to_string()));
    dummy_repos.push(("test-repository".to_string(), "git@github.com:user/test-repository.git".to_string()));
    dummy_repos.push(("sample-code".to_string(), "git@github.com:user/sample-code.git".to_string()));
    
    // Add repositories by category
    let categories = ["api", "web", "mobile", "backend", "frontend", "database", "utils", "tools", "docs", "test"];
    
    for i in 1..=97 {
        let category = categories[i % categories.len()];
        let name = format!("{}-project-{}", category, i);
        let url = format!("git@github.com:user/{}.git", name);
        dummy_repos.push((name, url));
    }
    
    dummy_repos
}

pub fn extract_repo_info(selection: &str) -> Option<(String, String, Option<String>)> {
    // Extract repository name and URL from selection
    if let Some((repo_name, url_part)) = selection.split_once(" (") {
        let url = url_part.trim_end_matches(")");
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

        Some((repo_name.to_string(), url.to_string(), browser_url))
    } else {
        None
    }
}
