use std::io::Write;
use std::process;
use termion;
use termion::input::TermRead;

/// Cleans up the terminal state before exiting
pub fn cleanup_terminal() {
    // Ensure terminal is in a clean state
    print!("{}{}", termion::screen::ToMainScreen, termion::cursor::Show);
    std::io::stdout().flush().unwrap();
    
    // Reset terminal attributes to ensure proper cleanup
    if let Ok(_) = termion::get_tty() {
        let _ = termion::async_stdin().keys().next(); // Consume any pending input
        let _ = termion::terminal_size(); // Force terminal refresh
    }
}

/// Sets up a Ctrl+C handler that works globally
pub fn setup_ctrl_c_handler() {
    // Use the ctrlc crate which works reliably across platforms
    ctrlc::set_handler(move || {
        cleanup_terminal();
        println!("\nReceived Ctrl+C, exiting...");
        process::exit(0);
    }).expect("Error setting Ctrl+C handler");
}
