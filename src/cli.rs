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
        .about("Pick GitHub repos by fuzzy filtering")
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
            Arg::new("force")
                .short('f')
                .long("force")
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
    let force_download = matches.get_flag("force");

    AppArgs {
        use_dummy,
        token,
        force_download,
    }
}
