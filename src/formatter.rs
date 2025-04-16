// Module for formatting repository information with icons and emojis

/// Formats a repository name with fork and private status icons
pub fn format_repo_name(name: &str, is_fork: bool, is_private: bool) -> String {
    let fork_icon = if is_fork { " 🍴" } else { "" };
    let private_icon = if is_private { " 🔒" } else { "" };

    format!("{}{}{}", name, fork_icon, private_icon)
}

/// Determines appropriate emoji based on repository name and description
pub fn get_category_emoji(name: &str, description: &str) -> &'static str {
    let text = format!("{} {}", name, description).to_lowercase();

    // Web/Frontend related
    if text.contains("web") || text.contains("frontend") || text.contains("html") ||
       text.contains("css") || text.contains("javascript") || text.contains("react") ||
       text.contains("vue") || text.contains("angular") {
        return "🌐";
    }

    // Backend related
    if text.contains("backend") || text.contains("server") || text.contains("api") ||
       text.contains("database") || text.contains("db") {
        return "🖥️";
    }

    // Mobile related
    if text.contains("mobile") || text.contains("android") || text.contains("ios") ||
       text.contains("app") || text.contains("flutter") || text.contains("swift") {
        return "📱";
    }

    // Data science/ML related
    if text.contains("data") || text.contains("machine learning") || text.contains("ml") ||
       text.contains("ai") || text.contains("analytics") || text.contains("tensorflow") ||
       text.contains("pytorch") {
        return "📊";
    }

    // Testing - check this first to avoid conflicts with 'script' in Tools
    if text.contains("test") || text.contains("spec") || text.contains("qa") {
        return "🧪";
    }

    // Tools/Utilities
    if text.contains("tool") || text.contains("util") || text.contains("cli") ||
       text.contains("command") || text.contains("script") {
        return "🔧";
    }

    // Documentation/Learning
    if text.contains("doc") || text.contains("tutorial") || text.contains("learn") ||
       text.contains("guide") || text.contains("book") {
        return "📚";
    }

    // Game development
    if text.contains("game") || text.contains("unity") || text.contains("unreal") ||
       text.contains("godot") {
        return "🎮";
    }

    // Default - no specific category identified
    ""
}

/// Formats a complete repository display string with name, description, and emojis
pub fn format_repository(name: &str, description: &str, is_fork: bool, is_private: bool) -> String {
    let formatted_name = format_repo_name(name, is_fork, is_private);
    let category_emoji = get_category_emoji(name, description);

    if description.is_empty() {
        if !category_emoji.is_empty() {
            format!("{} {}", formatted_name, category_emoji)
        } else {
            formatted_name
        }
    } else {
        format!("{} ({}) {}", formatted_name, description, category_emoji)
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
    fn test_get_category_emoji() {
        // Web/Frontend
        assert_eq!(get_category_emoji("web-project", "A frontend project"), "🌐");
        assert_eq!(get_category_emoji("react-app", "UI components"), "🌐");

        // Backend
        assert_eq!(get_category_emoji("api-server", "REST API"), "🖥️");
        assert_eq!(get_category_emoji("database-tool", "SQL utility"), "🖥️");

        // Mobile
        assert_eq!(get_category_emoji("ios-app", "Swift application"), "📱");
        assert_eq!(get_category_emoji("android-client", "Mobile app"), "📱");

        // Data science
        assert_eq!(get_category_emoji("data-analysis", "ML project"), "📊");
        assert_eq!(get_category_emoji("tensorflow-model", "AI experiment"), "📊");

        // Tools
        assert_eq!(get_category_emoji("cli-tool", "Command line utility"), "🔧");
        assert_eq!(get_category_emoji("script-collection", "Useful scripts"), "🔧");

        // Documentation
        assert_eq!(get_category_emoji("docs", "Project documentation"), "📚");
        assert_eq!(get_category_emoji("tutorial", "Learning materials"), "📚");

        // Games
        assert_eq!(get_category_emoji("game-engine", "2D game framework"), "🎮");
        assert_eq!(get_category_emoji("unity-project", "3D game"), "🎮");

        // Testing
        assert_eq!(get_category_emoji("test-suite", "QA tools"), "🧪");
        assert_eq!(get_category_emoji("spec-runner", "Testing framework"), "🧪");

        // No category
        assert_eq!(get_category_emoji("random-project", "Miscellaneous code"), "");
    }

    #[test]
    fn test_format_repository() {
        // Repository with description and category
        assert_eq!(
            format_repository("web-app", "Frontend application", false, false),
            "web-app (Frontend application) 🌐"
        );

        // Repository with description, category, and fork status
        assert_eq!(
            format_repository("forked-api", "Backend service", true, false),
            "forked-api 🍴 (Backend service) 🖥️"
        );

        // Repository with description, category, and private status
        assert_eq!(
            format_repository("mobile-app", "iOS client", false, true),
            "mobile-app 🔒 (iOS client) 📱"
        );

        // Repository with description, category, fork and private status
        assert_eq!(
            format_repository("game-demo", "Unity project", true, true),
            "game-demo 🍴 🔒 (Unity project) 🎮"
        );

        // Repository with no description but with category
        assert_eq!(
            format_repository("test-framework", "", false, false),
            "test-framework 🧪"
        );

        // Repository with no description and no category
        assert_eq!(
            format_repository("misc-code", "", false, false),
            "misc-code"
        );
    }
}
