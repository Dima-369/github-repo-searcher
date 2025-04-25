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

use tokio::sync::mpsc;

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

    // Create a channel for repository updates
    let (tx, mut rx) = mpsc::channel::<repository::RepoUpdateMessage>(100);

    // Create a channel for updating the fuzzy finder
    let (update_tx, mut update_rx) = mpsc::channel::<(Vec<String>, String)>(100);

    // Load repositories based on the mode (dummy or real)
    if args.use_dummy {
        // Use dummy data for testing
        repository::load_dummy_repositories(
            &mut all_repos,
            &mut github_username,
            &mut gitlab_username,
        );
    } else {
        // Load real repositories with background refresh
        repository::load_repositories_with_background_refresh(
            &args,
            &mut all_repos,
            &mut github_username,
            &mut gitlab_username,
            tx.clone(),
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

    // Spawn a task to handle repository updates
    let update_tx_clone = update_tx.clone();
    tokio::spawn(async move {

        while let Some(message) = rx.recv().await {
            match message {
                repository::RepoUpdateMessage::NewRepos { repos, github_username: _new_gh_username, gitlab_username: _new_gl_username } => {

                    // Format the new repositories
                    let new_choices: Vec<String> = repos
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

                    // Send update to the main thread
                    let _ = update_tx_clone.send((new_choices, String::new())).await;
                },
                repository::RepoUpdateMessage::Status(status) => {
                    // Send status update to the main thread
                    let _ = update_tx_clone.send((Vec::new(), status)).await;
                },
                repository::RepoUpdateMessage::Error(error) => {
                    // Send error update to the main thread
                    let _ = update_tx_clone.send((Vec::new(), format!("ERROR: {}", error))).await;
                },
                repository::RepoUpdateMessage::LoadingComplete => {
                    // Send completion message to the main thread
                    let _ = update_tx_clone.send((Vec::new(), "Repository loading complete".to_string())).await;

                    // Clear the message after a delay
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    let _ = update_tx_clone.send((Vec::new(), String::new())).await;
                }
            }
        }
    });

    // Run the fuzzy finder in a loop
    loop {
        // Check for updates before running the fuzzy finder
        while let Ok((new_items, status)) = update_rx.try_recv() {
            if !new_items.is_empty() {
                finder.update_items(new_items);
            }

            if !status.is_empty() {
                if status.starts_with("ERROR:") {
                    finder.set_error_message(Some(status));
                } else {
                    finder.set_status_message(Some(status));
                }
            } else {
                finder.set_status_message(None);
                finder.set_error_message(None);
            }
        }

        // Run the fuzzy finder
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
