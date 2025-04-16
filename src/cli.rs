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
    pub token: Option<String>,
    pub force_download: bool,
}

pub fn parse_args() -> AppArgs {
    let matches = Command::new("gh-url-picker")
        .version("0.1.0")
        .author("Your Name <you@example.com>")
        .about("Pick GitHub repos by fuzzy filtering with visual indicators for repository types")
        .arg(
            Arg::new("token")
                .short('t')
                .long("token")
                .value_name("GITHUB_TOKEN")
                .help("GitHub personal access token")
                .required_unless_present("dummy"),
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
    let token = if !use_dummy {
        matches.get_one::<String>("token").cloned()
    } else {
        None
    };

    // Check if force download is enabled
    let force_download = matches.get_flag("force-download");

    AppArgs {
        use_dummy,
        token,
        force_download,
    }
}
