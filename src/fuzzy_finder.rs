use std::io::{self, Write, stdout, stdin};
use std::thread;
use std::time::Duration;
use termion::input::TermRead;
use termion::event::Key;
use termion::raw::IntoRawMode;
use termion::screen::IntoAlternateScreen;
use termion::cursor;
use termion::color;
use termion::clear;
use termion::style;

use crate::filter;
use crate::INTERRUPTED;
use std::sync::atomic::Ordering;

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
            self.selected_index = if self.filtered_items.is_empty() { 0 } else { self.filtered_items.len() - 1 };
        }

        // Reset scroll offset if needed
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + self.max_display {
            self.scroll_offset = self.selected_index - self.max_display + 1;
        }
    }

    fn move_cursor_up(&mut self) {
        if !self.filtered_items.is_empty() {
            if self.selected_index > 0 {
                self.selected_index -= 1;

                // Adjust scroll offset if needed
                if self.selected_index < self.scroll_offset {
                    self.scroll_offset = self.selected_index;
                }
            }
        }
    }

    fn move_cursor_down(&mut self) {
        if !self.filtered_items.is_empty() {
            if self.selected_index < self.filtered_items.len() - 1 {
                self.selected_index += 1;

                // Adjust scroll offset if needed
                if self.selected_index >= self.scroll_offset + self.max_display {
                    self.scroll_offset = self.selected_index - self.max_display + 1;
                }
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
        let end_idx = std::cmp::min(self.scroll_offset + display_count, self.filtered_items.len());

        // Display items
        for i in self.scroll_offset..end_idx {
            let item = &self.filtered_items[i];

            // Calculate available width for text (accounting for the prefix)
            let prefix_len = 2; // Both "> " and "  " are 2 characters
            let available_width = width as usize - prefix_len;

            // Truncate item text if it's too long
            let display_text = if item.len() > available_width {
                // Truncate and add ellipsis
                format!("{:.width$}…", item, width = available_width - 1)
            } else {
                item.clone()
            };

            // Highlight selected item
            if i == self.selected_index {
                write!(screen, "{}{}> {}{}", color::Fg(color::Green), style::Bold, display_text, style::Reset)?;
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
        write!(screen, "{}", cursor::Goto(1, height - 2))?;

        // Create the status text with count
        let count_text = format!("{}/{}", self.filtered_items.len(), self.items.len());

        // Display status line at the bottom (format: "12/12 ───────────────")
        write!(
            screen,
            "{}{} {}{}\r\n",
            color::Fg(color::Yellow),
            count_text,
            color::Fg(color::Blue),
            "─".repeat(width as usize - count_text.len() - 1)
        )?;
        write!(screen, "{}", style::Reset)?;

        // Display prompt at the bottom with input text on the same line
        write!(screen, "{}>{} ", color::Fg(color::Blue), style::Reset)?;

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
            write!(screen, "{}", cursor::Goto(width, height - 1))?;
        } else {
            // Otherwise, position cursor at the current position (after the prompt)
            write!(screen, "{}", cursor::Goto(self.cursor_pos as u16 + 3, height - 1))?;
        }

        screen.flush()?;
        Ok(())
    }

    pub fn run(&mut self) -> Option<String> {
        // Set up terminal
        let mut screen = stdout().into_raw_mode().unwrap()
            .into_alternate_screen().unwrap();

        // Initial render
        self.render(&mut screen).unwrap();

        // Process input
        let stdin = stdin();
        let mut keys = stdin.keys();

        loop {
            // Check if interrupted
            if INTERRUPTED.load(Ordering::SeqCst) {
                return None;
            }

            // Process key input
            if let Some(Ok(key)) = keys.next() {
                match key {
                    Key::Char('\n') | Key::Char('\r') => {
                        // Return selected item
                        if !self.filtered_items.is_empty() {
                            return Some(self.filtered_items[self.selected_index].clone());
                        }
                    },
                    Key::Char(c) => {
                        // Add character to query
                        self.query.push(c);
                        self.cursor_pos += 1;
                        self.update_filter();
                    },
                    Key::Backspace => {
                        // Remove character from query
                        if !self.query.is_empty() && self.cursor_pos > 0 {
                            self.query.pop();
                            self.cursor_pos -= 1;
                            self.update_filter();
                        }
                    },
                    Key::Up => {
                        self.move_cursor_up();
                    },
                    Key::Down => {
                        self.move_cursor_down();
                    },
                    Key::Ctrl('c') => {
                        return None;
                    },
                    Key::Esc => {
                        // Exit with code 0 on Escape
                        println!("\nExiting due to Escape key press");
                        unsafe { libc::_exit(0); }
                    },
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
