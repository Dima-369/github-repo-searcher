use std::process::Command;

pub enum ClipboardContent {
    GitCloneCommand(String),
    SshUrl(String),
}

pub fn copy_to_clipboard(content: ClipboardContent) -> Result<String, String> {
    let (text, description) = match content {
        ClipboardContent::GitCloneCommand(cmd) => (cmd, "git clone command"),
        ClipboardContent::SshUrl(url) => (url, "SSH URL"),
    };

    if cfg!(target_os = "macos") {
        let output = Command::new("pbcopy")
            .arg(&text)
            .output();

        match output {
            Ok(_) => Ok(format!("Copied {} to clipboard: {}", description, text)),
            Err(e) => Err(format!("Failed to copy to clipboard: {}", e))
        }
    } else if cfg!(target_os = "linux") {
        let output = Command::new("xclip")
            .arg("-selection")
            .arg("clipboard")
            .arg(&text)
            .output();

        match output {
            Ok(_) => Ok(format!("Copied {} to clipboard: {}", description, text)),
            Err(e) => Err(format!("Failed to copy to clipboard: {}", e))
        }
    } else {
        Ok(format!("Clipboard not supported on this platform. {}: {}", description, text))
    }
}

pub fn open_in_browser(url: &str) -> Result<String, String> {
    if cfg!(target_os = "macos") {
        let output = Command::new("open")
            .arg(url)
            .output();

        match output {
            Ok(_) => Ok(format!("Opened in browser: {}", url)),
            Err(e) => Err(format!("Failed to open browser: {}", e))
        }
    } else if cfg!(target_os = "linux") {
        let output = Command::new("xdg-open")
            .arg(url)
            .output();

        match output {
            Ok(_) => Ok(format!("Opened in browser: {}", url)),
            Err(e) => Err(format!("Failed to open browser: {}", e))
        }
    } else {
        Ok(format!("Opening browser not supported on this platform. URL: {}", url))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_content_git_clone() {
        let content = ClipboardContent::GitCloneCommand("git clone test-repo".to_string());
        let (text, desc) = match content {
            ClipboardContent::GitCloneCommand(cmd) => (cmd, "git clone command"),
            _ => panic!("Wrong enum variant"),
        };

        assert_eq!(text, "git clone test-repo");
        assert_eq!(desc, "git clone command");
    }

    #[test]
    fn test_clipboard_content_ssh_url() {
        let content = ClipboardContent::SshUrl("git@github.com:user/repo.git".to_string());
        let (text, desc) = match content {
            ClipboardContent::SshUrl(url) => (url, "SSH URL"),
            _ => panic!("Wrong enum variant"),
        };

        assert_eq!(text, "git@github.com:user/repo.git");
        assert_eq!(desc, "SSH URL");
    }

    #[test]
    fn test_copy_to_clipboard_unsupported_platform() {
        // This test only works when run on a platform that's neither macOS nor Linux
        if !cfg!(target_os = "macos") && !cfg!(target_os = "linux") {
            let content = ClipboardContent::SshUrl("test-url".to_string());
            let result = copy_to_clipboard(content);
            assert!(result.is_ok());
            assert!(result.unwrap().contains("Clipboard not supported"));
        }
    }

    #[test]
    fn test_open_in_browser_unsupported_platform() {
        // This test only works when run on a platform that's neither macOS nor Linux
        if !cfg!(target_os = "macos") && !cfg!(target_os = "linux") {
            let result = open_in_browser("https://example.com");
            assert!(result.is_ok());
            assert!(result.unwrap().contains("Opening browser not supported"));
        }
    }
}
