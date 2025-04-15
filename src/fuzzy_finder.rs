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
        // Clear screen
        write!(screen, "{}{}", clear::All, cursor::Goto(1, 1))?;

        // Display header
        write!(screen, "{}{}> {}{}", color::Fg(color::Blue), style::Bold, self.query, style::Reset)?;
        write!(screen, "{}\r\n", cursor::Goto(self.cursor_pos as u16 + 3, 1))?;

        // Display items
        let display_count = std::cmp::min(self.max_display, self.filtered_items.len());
        let end_idx = std::cmp::min(self.scroll_offset + display_count, self.filtered_items.len());

        for i in self.scroll_offset..end_idx {
            let item = &self.filtered_items[i];

            // Highlight selected item
            if i == self.selected_index {
                write!(screen, "{}{}{} {}{}", color::Fg(color::Green), style::Bold, ">", item, style::Reset)?;
            } else {
                write!(screen, "  {}", item)?;
            }

            write!(screen, "\r\n")?;
        }

        // Display status line
        write!(screen, "{}{}{}\r\n", color::Fg(color::Blue), "-".repeat(50), style::Reset)?;
        write!(screen, "{}[{}/{}] Press Ctrl+C to quit, Enter to select{}",
               color::Fg(color::Yellow),
               self.filtered_items.len(),
               self.items.len(),
               style::Reset)?;

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
