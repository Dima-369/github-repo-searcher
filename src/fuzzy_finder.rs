use std::io::{self, stdin, stdout, Write};
use std::process;
use std::thread;
use std::time::Duration;
use termion::clear;
use termion::color;
use termion::cursor;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::IntoAlternateScreen;
use termion::style;

use crate::filter;

// Custom UI for displaying and filtering repositories
pub struct FuzzyFinder {
    items: Vec<String>,
    filtered_items: Vec<String>,
    query: String,
    cursor_pos: usize,
    selected_index: usize,
    max_display: usize,
    scroll_offset: usize,
}

impl FuzzyFinder {
    // Helper method to clean up terminal state
    fn cleanup_terminal<W: Write>(screen: &mut W) {
        write!(screen, "{}{}", termion::screen::ToMainScreen, cursor::Show).unwrap();
        screen.flush().unwrap();
    }

    // Helper method to exit the program
    fn exit_program<W: Write>(screen: &mut W, message: &str) -> ! {
        Self::cleanup_terminal(screen);
        let _ = screen; // Mark screen as used without trying to drop the reference
        println!("{}", message);
        process::exit(0);
    }

    pub fn new(items: Vec<String>) -> Self {
        let filtered_items = items.clone();
        let max_display = 10; // Number of items to display at once

        Self {
            items,
            filtered_items,
            query: String::new(),
            cursor_pos: 0,
            selected_index: 0,
            max_display,
            scroll_offset: 0,
        }
    }

    fn update_filter(&mut self) {
        // Use the filter_human function to filter items based on query
        self.filtered_items = filter::filter_human(&self.items, &self.query, |s| s.clone());

        // Reset selection if it's out of bounds
        if self.selected_index >= self.filtered_items.len() {
            self.selected_index = if self.filtered_items.is_empty() {
                0
            } else {
                self.filtered_items.len() - 1
            };
        }

        // Reset scroll offset if needed
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + self.max_display {
            self.scroll_offset = self.selected_index - self.max_display + 1;
        }
    }

    fn move_cursor_up(&mut self) {
        if !self.filtered_items.is_empty() && self.selected_index > 0 {
            self.selected_index -= 1;

            // Adjust scroll offset if needed
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
            }
        }
    }

    fn move_cursor_down(&mut self) {
        if !self.filtered_items.is_empty() && self.selected_index < self.filtered_items.len() - 1 {
            self.selected_index += 1;

            // Adjust scroll offset if needed
            if self.selected_index >= self.scroll_offset + self.max_display {
                self.scroll_offset = self.selected_index - self.max_display + 1;
            }
        }
    }

    fn render<W: Write>(&self, screen: &mut W) -> io::Result<()> {
        // Get terminal size
        let (width, height) = termion::terminal_size().unwrap_or((80, 24));

        // Clear screen
        write!(screen, "{}{}", clear::All, cursor::Goto(1, 1))?;

        // Calculate available space for items (accounting for prompt and status lines)
        let available_lines = height as usize - 3; // Prompt line (with input) + status line + separator line

        // Adjust max_display based on available space
        let display_count = std::cmp::min(available_lines, self.filtered_items.len());
        let end_idx = std::cmp::min(
            self.scroll_offset + display_count,
            self.filtered_items.len(),
        );

        // Display items
        for i in self.scroll_offset..end_idx {
            let item = &self.filtered_items[i];

            // Calculate available width for text (accounting for the prefix)
            let prefix_len = 2; // Both "> " and "  " are 2 characters
            let available_width = width as usize - prefix_len - 5; // Extra buffer for emojis and safety

            // Truncate item text if it's too long
            let display_text = if item.chars().count() > available_width {
                // Truncate and add ellipsis, being careful with multibyte characters like emojis
                let mut truncated = String::new();
                let mut char_count = 0;

                for c in item.chars() {
                    if char_count >= available_width - 1 {
                        break;
                    }
                    truncated.push(c);
                    char_count += 1;
                }

                format!("{truncated}…")
            } else {
                item.clone()
            };

            // Highlight selected item
            if i == self.selected_index {
                write!(
                    screen,
                    "{}{}> {}{}",
                    color::Fg(color::Green),
                    style::Bold,
                    display_text,
                    style::Reset
                )?;
            } else {
                write!(screen, "  {}", display_text)?;
            }

            write!(screen, "\r\n")?;
        }

        // Fill any remaining lines with empty space
        let empty_lines = height as usize - 3 - (end_idx - self.scroll_offset);
        for _ in 0..empty_lines {
            write!(screen, "\r\n")?;
        }

        // Position cursor at the bottom of the screen for status line
        write!(screen, "{}", cursor::Goto(1, height - 1))?;

        // Create the status text with count
        let count_text = format!("{}/{}", self.filtered_items.len(), self.items.len());

        // Display status line at the bottom (format: "12/12 ───────────────")
        write!(
            screen,
            "{}{} {}{}",
            color::Fg(color::Yellow),
            count_text,
            color::Fg(color::Blue),
            "─".repeat(width as usize - count_text.len() - 1)
        )?;
        write!(screen, "{}", style::Reset)?;

        // Display prompt at the bottom with input text on the same line
        write!(screen, "\r\n{}>{} ", color::Fg(color::Blue), style::Reset)?;

        // Display the input text on the same line as the prompt
        if !self.query.is_empty() {
            // Truncate query if it's too long for the terminal width
            // Account for the prompt (2 characters: '>' and space)
            let available_width = width as usize - 2;
            let display_query = if self.query.len() > available_width {
                // Show the last part of the query that fits in the terminal
                let start_pos = self.query.len() - available_width + 1;
                format!("…{}", &self.query[start_pos..])
            } else {
                self.query.clone()
            };
            write!(screen, "{}", display_query)?;
        }

        // Position cursor at the right position in the input line
        let available_width = width as usize - 2; // Account for '>' and space
        if self.query.len() > available_width {
            // If text is truncated, position cursor at the end of visible text
            write!(screen, "{}", cursor::Goto(width, height))?;
        } else {
            // Otherwise, position cursor at the current position (after the prompt)
            write!(
                screen,
                "{}",
                cursor::Goto(self.cursor_pos as u16 + 3, height)
            )?;
        }

        screen.flush()?;
        Ok(())
    }

    pub fn run(&mut self) -> Option<String> {
        // Set up terminal
        let mut screen = stdout()
            .into_raw_mode()
            .unwrap()
            .into_alternate_screen()
            .unwrap();

        // Hide cursor and perform initial render
        write!(screen, "{}", cursor::Hide).unwrap();
        screen.flush().unwrap();
        self.render(&mut screen).unwrap();

        // Process input
        let stdin = stdin();
        let mut keys = stdin.keys();

        loop {
            // Process key input
            if let Some(Ok(key)) = keys.next() {
                match key {
                    Key::Char('\n') | Key::Char('\r') => {
                        // Return selected item but don't exit the program
                        if !self.filtered_items.is_empty() {
                            // Store the selected item
                            let selected = self.filtered_items[self.selected_index].clone();

                            // Properly restore terminal state before returning
                            Self::cleanup_terminal(&mut screen);
                            let _ = screen; // Mark screen as used without trying to drop the reference

                            // Return the selected item to be processed
                            return Some(selected);
                        }
                    }
                    Key::Char(c) => {
                        // Add character to query
                        self.query.push(c);
                        self.cursor_pos += 1;
                        self.update_filter();
                    }
                    Key::Backspace => {
                        // Remove character from query
                        if !self.query.is_empty() && self.cursor_pos > 0 {
                            self.query.pop();
                            self.cursor_pos -= 1;
                            self.update_filter();
                        }
                    }
                    Key::Up => {
                        self.move_cursor_up();
                    }
                    Key::Down => {
                        self.move_cursor_down();
                    }
                    Key::Ctrl('c') => {
                        Self::exit_program(&mut screen, "\nExiting...");
                    }
                    Key::Esc => {
                        Self::exit_program(&mut screen, "\nExiting...");
                    }
                    _ => {}
                }

                // Re-render after each key press
                self.render(&mut screen).unwrap();
            }

            // Small sleep to prevent CPU hogging
            thread::sleep(Duration::from_millis(10));
        }
    }
}
