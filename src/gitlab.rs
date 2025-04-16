use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::Deserialize;
use std::io::Write;

// Define our Repository type to match GitHub's format
pub type Repository = (String, String, String, String, bool, bool); // (name, ssh_url, description, owner, is_fork, is_private)

// GitLab API response structures
#[derive(Debug, Deserialize, Clone)]
struct GitLabProject {
    #[allow(dead_code)]
    id: u64,
    name: String,
    description: Option<String>,
    ssh_url_to_repo: String,
    #[allow(dead_code)]
    namespace: GitLabNamespace,
    forked_from_project: Option<GitLabForkedFrom>,
    visibility: String,
}

#[derive(Debug, Deserialize, Clone)]
struct GitLabNamespace {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    path: String,
}

#[derive(Debug, Deserialize, Clone)]
struct GitLabForkedFrom {
    #[allow(dead_code)]
    id: u64,
}

// Helper function to convert GitLab project to our Repository type
fn convert_project(project: GitLabProject, username: &str) -> Repository {
    (
        project.name,
        project.ssh_url_to_repo,
        project.description.unwrap_or_default(),
        username.to_string(),
        project.forked_from_project.is_some(),
        project.visibility != "public",
    )
}

// Helper function to update progress display
fn update_progress(page_count: usize, repos_count: usize) {
    print!("\r                                                  "); // Clear the line
    print!("\rFetched page {} ({} repos so far)... ", page_count, repos_count);
    std::io::stdout().flush().unwrap();
}

pub async fn fetch_repos(token: &str) -> Result<(String, Vec<Repository>), Box<dyn std::error::Error>> {
    print!("Fetching GitLab user information... ");
    std::io::stdout().flush().unwrap();

    // Create HTTP client with authorization header
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );

    // Get user information
    let response = client
        .get("https://gitlab.com/api/v4/user")
        .headers(headers.clone())
        .send()
        .await?;

    // Check if response is successful
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await?;
        return Err(format!("GitLab API error: {} - {}", status, text).into());
    }

    let user: serde_json::Value = response.json().await?;

    let username = user["username"]
        .as_str()
        .ok_or("Failed to get GitLab username. Please check your GitLab token.")?
        .to_string();

    println!("✓"); // Show checkmark on its own line
    print!("Fetching repositories for GitLab user {}... ", username);
    std::io::stdout().flush().unwrap();

    let mut all_repos = Vec::new();
    let mut page_count = 1;
    let per_page = 100; // Maximum allowed per page

    // Fetch first page
    let response = client
        .get("https://gitlab.com/api/v4/projects")
        .headers(headers.clone())
        .query(&[
            ("membership", "true"), // Get projects user is a member of
            ("per_page", &per_page.to_string()),
            ("page", &page_count.to_string()),
        ])
        .send()
        .await?;

    // Check if response is successful
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await?;
        return Err(format!("GitLab API error: {} - {}", status, text).into());
    }

    // Parse the response as JSON
    let mut projects: Vec<GitLabProject> = response.json().await?;

    // Add repos from the first page
    all_repos.extend(
        projects.clone()
            .into_iter()
            .map(|project| convert_project(project, &username))
    );

    update_progress(page_count, all_repos.len());

    // Fetch all remaining pages
    while !projects.is_empty() && projects.len() == per_page {
        // Add a small sleep to allow Ctrl+C to be processed
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        page_count += 1;

        let response = client
            .get("https://gitlab.com/api/v4/projects")
            .headers(headers.clone())
            .query(&[
                ("membership", "true"),
                ("per_page", &per_page.to_string()),
                ("page", &page_count.to_string()),
            ])
            .send()
            .await?;

        // Check if response is successful
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(format!("GitLab API error: {} - {}", status, text).into());
        }

        // Parse the response as JSON
        projects = response.json().await?;

        all_repos.extend(
            projects.clone()
                .into_iter()
                .map(|project| convert_project(project, &username))
        );

        update_progress(page_count, all_repos.len());
    }

    println!("✓"); // Show checkmark on its own line
    println!("Fetched {} GitLab repositories from {} pages", all_repos.len(), page_count);
    Ok((username, all_repos))
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
    let url = format!("git@gitlab.com:{}/{}.git", username, repo_name);

    // Extract GitLab repo path for browser URL
    let browser_url = Some(format!("https://gitlab.com/{}/{}", username, repo_name));

    Some((repo_name.to_string(), url, browser_url))
}
