//! Module for formatting repository information with icons
//!
//! This module provides functions for formatting repository names and descriptions
//! with visual indicators to help quickly identify their type.
//!
//! # Repository Display Format
//!
//! ## Status Indicators (at the end of repository name)
//!
//! - 🍴 - Fork of another repository
//! - 🔒 - Private repository

/// Formats a repository name with fork and private status icons
pub fn format_repo_name(name: &str, is_fork: bool, is_private: bool) -> String {
    let fork_icon = if is_fork { " 🍴" } else { "" };
    let private_icon = if is_private { " 🔒" } else { "" };

    format!("{}{}{}", name, fork_icon, private_icon)
}



/// Formats a complete repository display string with name and description
pub fn format_repository(name: &str, description: &str, is_fork: bool, is_private: bool) -> String {
    let formatted_name = format_repo_name(name, is_fork, is_private);

    if description.is_empty() {
        formatted_name
    } else {
        // Trim the description before formatting
        let trimmed_description = description.trim();
        format!("{} ({})", formatted_name, trimmed_description)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_repo_name() {
        // Regular repository (no icons)
        assert_eq!(format_repo_name("normal-repo", false, false), "normal-repo");

        // Forked repository
        assert_eq!(format_repo_name("forked-repo", true, false), "forked-repo 🍴");

        // Private repository
        assert_eq!(format_repo_name("private-repo", false, true), "private-repo 🔒");

        // Both forked and private
        assert_eq!(format_repo_name("private-fork", true, true), "private-fork 🍴 🔒");
    }



    #[test]
    fn test_format_repository() {
        // Repository with description
        assert_eq!(
            format_repository("web-app", "Frontend application", false, false),
            "web-app (Frontend application)"
        );

        // Repository with description and fork status
        assert_eq!(
            format_repository("forked-api", "Backend service", true, false),
            "forked-api 🍴 (Backend service)"
        );

        // Repository with description and private status
        assert_eq!(
            format_repository("mobile-app", "iOS client", false, true),
            "mobile-app 🔒 (iOS client)"
        );

        // Repository with description, fork and private status
        assert_eq!(
            format_repository("game-demo", "Unity project", true, true),
            "game-demo 🍴 🔒 (Unity project)"
        );

        // Repository with no description
        assert_eq!(
            format_repository("test-framework", "", false, false),
            "test-framework"
        );

        // Repository with no description but with status icons
        assert_eq!(
            format_repository("private-fork", "", true, true),
            "private-fork 🍴 🔒"
        );

        // Repository with description containing extra whitespace
        assert_eq!(
            format_repository("whitespace-test", "  Description with extra spaces  ", false, false),
            "whitespace-test (Description with extra spaces)"
        );
    }
}
