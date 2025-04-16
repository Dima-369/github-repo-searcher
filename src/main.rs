use std::error::Error;
use std::process;

mod browser;
mod cache;
mod cli;
mod filter;
mod formatter;
mod fuzzy_finder;
mod github;
mod gitlab;
mod repository;
mod terminal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Set up global Ctrl+C handler
    terminal::setup_ctrl_c_handler();

    // Parse command line arguments
    let args = cli::parse_args();

    // Use the RepoData struct from the cache module
    use cache::RepoData;

    // Initialize repository data and usernames
    let mut all_repos: Vec<RepoData> = Vec::new();
    let mut github_username = String::new();
    let mut gitlab_username = String::new();

    // Load repositories based on the mode (dummy or real)
    if args.use_dummy {
        // Use dummy data for testing
        repository::load_dummy_repositories(
            &mut all_repos,
            &mut github_username,
            &mut gitlab_username,
        );
    } else {
        // Load real repositories (from cache or API)
        repository::load_real_repositories(
            &args,
            &mut all_repos,
            &mut github_username,
            &mut gitlab_username,
        )
        .await?;
    }

    // Print summary of repositories found
    let github_count = all_repos
        .iter()
        .filter(|r| matches!(r.source, formatter::RepoSource::GitHub))
        .count();
    let gitlab_count = all_repos
        .iter()
        .filter(|r| matches!(r.source, formatter::RepoSource::GitLab))
        .count();
    println!(
        "Found {} repositories: {} from GitHub, {} from GitLab",
        all_repos.len(),
        github_count,
        gitlab_count
    );

    // Create formatted choices for the fuzzy finder
    let choices: Vec<String> = all_repos
        .iter()
        .map(|repo| {
            formatter::format_repository(
                &repo.name,
                &repo.description,
                repo.is_fork,
                repo.is_private,
                repo.source,
            )
        })
        .collect();

    // Create the fuzzy finder
    let mut finder = fuzzy_finder::FuzzyFinder::new(choices);

    // Run the fuzzy finder in a loop
    loop {
        let selection = match finder.run() {
            Some(selected) => selected,
            None => {
                terminal::cleanup_terminal();
                println!("No selection made");
                process::exit(0);
            }
        };

        // Process the selected repository
        if let Err(e) =
            repository::process_repository_selection(&selection, &github_username, &gitlab_username)
                .await
        {
            eprintln!("Error processing repository: {}", e);
        }
    }

    // The loop above never exits normally, only through Ctrl+C or Esc
    // which call process::exit(0), so this is unreachable
}
