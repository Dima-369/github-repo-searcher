# GitHub Repository Searcher

A command-line tool for quickly searching and opening your GitHub repositories using fuzzy search.

## Features

- Fuzzy search through all your GitHub repositories
- Repository caching for faster startup (30-minute expiration)
- Visual indicators for repository types (fork/private)
- Direct browser opening of selected repositories

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Use with your GitHub token
gh-url-picker --token YOUR_GITHUB_TOKEN

# Force refresh the repository cache
gh-url-picker --token YOUR_GITHUB_TOKEN --force-download

# Use dummy repositories for testing
gh-url-picker --dummy
```

## Repository Display Format

Repositories are displayed with visual indicators to help you quickly identify their type:

### Status Indicators (at the end of repository name)

- 🍴 - Fork of another repository
- 🔒 - Private repository

### Examples

```
repo-name 🍴 (A forked repository)
web-project (A frontend application)
private-api 🔒 (Internal API service)
game-demo 🍴 🔒 (Private fork of a game)
```

## Keyboard Controls

- **Up/Down Arrow**: Navigate through repositories
- **Enter**: Select repository and open in browser
- **Ctrl+C or Esc**: Exit the program

## Bugs

- `Ctrl-C` does not work when downloading repository info

## Future Plans

- Add GitLab support