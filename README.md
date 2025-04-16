# Repository Searcher

A command-line tool for quickly searching and opening your GitHub and GitLab repositories using fuzzy search.

## Features

- Fuzzy search through all your GitHub and GitLab repositories
- Support for both GitHub and GitLab APIs
- Repository caching for faster startup (30-minute expiration)
- Visual indicators for repository types (fork/private) and source (GitHub/GitLab)
- Direct browser opening of selected repositories

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Use with your GitHub token
repo-url-picker --github-token YOUR_GITHUB_TOKEN

# Use with your GitLab token
repo-url-picker --gitlab-token YOUR_GITLAB_TOKEN

# Use with both GitHub and GitLab tokens
repo-url-picker --github-token YOUR_GITHUB_TOKEN --gitlab-token YOUR_GITLAB_TOKEN

# Force refresh the repository cache
repo-url-picker --github-token YOUR_GITHUB_TOKEN --force-download

# Use dummy repositories for testing
repo-url-picker --dummy
```

## Repository Display Format

Repositories are displayed with visual indicators to help you quickly identify their type:

### Status Indicators

- `(fork)` or `(fork: description)` - Fork of another repository
- 🔒 - Private repository
- `[GH]` - GitHub repository
- `[GL]` - GitLab repository

### Examples

```
repo-name [GH] (fork: A forked repository)
web-project [GH] (A frontend application)
private-api 🔒 [GH] (Internal API service)
game-demo 🔒 [GL] (fork: Private fork of a game)
api-client [GL] (A GitLab API client)
```

## Keyboard Controls

- **Up/Down Arrow**: Navigate through repositories
- **Enter**: Select repository and open in browser (program continues running)
- **Ctrl+C or Esc**: Exit the program

## Bugs

- `Ctrl-C` does not work when downloading repository info

## Future Plans

- Improve caching to store repositories from multiple sources