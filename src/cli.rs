use clap::{Arg, Command};

pub struct AppArgs {
    pub use_dummy: bool,
    pub token: Option<String>,
    pub username: String,
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
            Arg::new("username")
                .short('u')
                .long("username")
                .value_name("GITHUB_USERNAME")
                .help("GitHub username")
                .default_value("dima-369")
        )
        .get_matches();

    // Check if dummy mode is enabled
    let use_dummy = matches.get_flag("dummy");
    let token = if !use_dummy {
        matches.get_one::<String>("token").cloned()
    } else {
        None
    };

    // Get the GitHub username
    let username = matches.get_one::<String>("username")
        .cloned()
        .unwrap_or_else(|| "dima-369".to_string());

    AppArgs {
        use_dummy,
        token,
        username,
    }
}
