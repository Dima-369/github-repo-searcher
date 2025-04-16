use std::process;
use std::time::Duration;
use tokio;

/// Opens a URL in the default browser
pub async fn open_in_browser(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\nOpening URL in browser: {}", url);
    
    // Open URL in browser based on the operating system
    #[cfg(target_os = "macos")]
    {
        process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to open URL in browser: {}", e))?
            .wait()
            .map_err(|e| format!("Failed to wait on browser process: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        process::Command::new("cmd")
            .args(["/c", "start", url])
            .spawn()
            .map_err(|e| format!("Failed to open URL in browser: {}", e))?
            .wait()
            .map_err(|e| format!("Failed to wait on browser process: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to open URL in browser: {}", e))?
            .wait()
            .map_err(|e| format!("Failed to wait on browser process: {}", e))?;
    }

    // Small delay to ensure operation completes
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    Ok(())
}
