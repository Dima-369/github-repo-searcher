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
