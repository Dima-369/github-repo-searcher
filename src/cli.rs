//! Command-line interface for the GitHub Repository Searcher
//!
//! This module handles parsing command-line arguments and provides
//! the main entry point for the application.
//!
//! # Repository Display Format
//!
//! Repositories are displayed with visual indicators to help quickly identify their type:
//!
//! ## Status Indicators
//!
//! - (fork) or (fork: description) - Fork of another repository
//! - 🔒 - Private repository (shown at the end of repository name)

use clap::{Arg, Command};

pub struct AppArgs {
    pub use_dummy: bool,
    pub github_token: Option<String>,
    pub gitlab_token: Option<String>,
    pub force_download: bool,
}

pub fn parse_args() -> AppArgs {
    let matches = Command::new("repo-url-picker")
        .version("0.1.0")
        .author("Your Name <you@example.com>")
        .about("Pick GitHub and GitLab repos by fuzzy filtering with visual indicators for repository types")
        .arg(
            Arg::new("github-token")
                .short('g')
                .long("github-token")
                .value_name("GITHUB_TOKEN")
                .help("GitHub personal access token")
                .conflicts_with("dummy"),
        )
        .arg(
            Arg::new("gitlab-token")
                .short('l')
                .long("gitlab-token")
                .value_name("GITLAB_TOKEN")
                .help("GitLab personal access token")
                .conflicts_with("dummy"),
        )
        .arg(
            Arg::new("dummy")
                .short('d')
                .long("dummy")
                .help("Use 100 dummy repositories for testing the UI")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("force-download")
                .short('f')
                .long("force-download")
                .help("Force download repositories from GitHub, ignoring cache")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Check if dummy mode is enabled
    let use_dummy = matches.get_flag("dummy");

    // Get GitHub and GitLab tokens
    let github_token = if !use_dummy {
        matches.get_one::<String>("github-token").cloned()
    } else {
        None
    };

    let gitlab_token = if !use_dummy {
        matches.get_one::<String>("gitlab-token").cloned()
    } else {
        None
    };

    // Validate that at least one token is provided if not in dummy mode
    if !use_dummy && github_token.is_none() && gitlab_token.is_none() {
        eprintln!("Error: At least one of --github-token or --gitlab-token must be provided");
        eprintln!("       Alternatively, use --dummy for testing with sample data");
        std::process::exit(1);
    }

    // Check if force download is enabled
    let force_download = matches.get_flag("force-download");

    AppArgs {
        use_dummy,
        github_token,
        gitlab_token,
        force_download,
    }
}
