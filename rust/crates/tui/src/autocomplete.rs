//! Slash command autocomplete — shows filtered suggestions when user types "/".

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget};

use commands::SlashCommandSpec;

/// Autocomplete state for slash commands.
pub struct SlashSuggestions {
    /// Whether the popup is visible.
    pub active: bool,
    /// Filtered suggestions based on current input.
    pub items: Vec<&'static SlashCommandSpec>,
    /// Currently selected index.
    pub selected: usize,
    /// The filter text (what the user typed after /).
    pub filter: String,
}

impl Default for SlashSuggestions {
    fn default() -> Self {
        Self {
            active: false,
            items: Vec::new(),
            selected: 0,
            filter: String::new(),
        }
    }
}

impl SlashSuggestions {
    /// Update suggestions based on current input text.
    /// Call this every time the input changes.
    pub fn update(&mut self, input_text: &str) {
        if input_text.starts_with('/') && !input_text.contains(' ') {
            self.filter = input_text[1..].to_string();
            let filter_lower = self.filter.to_ascii_lowercase();
            self.items = commands::slash_command_specs()
                .iter()
                .filter(|spec| {
                    if filter_lower.is_empty() {
                        return true;
                    }
                    spec.name.starts_with(&filter_lower)
                        || spec.aliases.iter().any(|a| a.starts_with(&filter_lower))
                })
                .collect();
            self.active = !self.items.is_empty();
            // Clamp selection
            if self.selected >= self.items.len() {
                self.selected = self.items.len().saturating_sub(1);
            }
        } else {
            self.active = false;
            self.items.clear();
            self.selected = 0;
        }
    }

    /// Move selection down.
    pub fn next(&mut self) {
        if !self.items.is_empty() {
            self.selected = (self.selected + 1) % self.items.len();
        }
    }

    /// Move selection up.
    pub fn prev(&mut self) {
        if !self.items.is_empty() {
            self.selected = self.selected.checked_sub(1).unwrap_or(self.items.len() - 1);
        }
    }

    /// Get the selected command name (with /).
    pub fn accept(&self) -> Option<String> {
        self.items.get(self.selected).map(|spec| {
            let mut cmd = format!("/{}", spec.name);
            if spec.argument_hint.is_some() {
                cmd.push(' ');
            }
            cmd
        })
    }

    /// Dismiss the autocomplete popup.
    pub fn dismiss(&mut self) {
        self.active = false;
        self.items.clear();
        self.selected = 0;
    }

    /// Render the autocomplete popup above the input area.
    /// Supports scrolling — shows a window of up to 12 visible items.
    pub fn render(&self, input_area: Rect, buf: &mut Buffer) {
        if !self.active || self.items.is_empty() {
            return;
        }

        let max_visible: usize = 12;
        let visible_count = self.items.len().min(max_visible);
        let popup_height = (visible_count as u16 + 2).min(14); // +2 for borders
        let popup_width = input_area.width.min(70);
        let popup_x = input_area.x;
        let popup_y = input_area.y.saturating_sub(popup_height);

        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        // Clear background
        Clear.render(popup_area, buf);

        // Build title with scroll indicator
        let title_text = if self.items.len() > max_visible {
            format!(" Commands ({}/{}) ", self.selected + 1, self.items.len())
        } else {
            " Commands ".to_string()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(50, 130, 255)))
            .title(Line::from(Span::styled(
                title_text,
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )));

        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        // Calculate visible window (scroll so selected item is always visible)
        let scroll_offset = if self.selected >= max_visible {
            self.selected - max_visible + 1
        } else {
            0
        };
        let visible_items: Vec<(usize, &&SlashCommandSpec)> = self.items
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(max_visible)
            .collect();

        let max_name_w = visible_items.iter().map(|(_, s)| s.name.len()).max().unwrap_or(10);

        let lines: Vec<Line<'_>> = visible_items
            .iter()
            .map(|(i, spec)| {
                let is_selected = *i == self.selected;
                let name_pad = max_name_w + 2 - spec.name.len();

                let name_style = if is_selected {
                    Style::default().fg(Color::Rgb(255, 165, 0)).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let desc_style = if is_selected {
                    Style::default().fg(Color::Indexed(252))
                } else {
                    Style::default().fg(Color::Indexed(245))
                };
                let indicator = if is_selected { "▸ " } else { "  " };

                Line::from(vec![
                    Span::styled(indicator, Style::default().fg(Color::Rgb(255, 165, 0))),
                    Span::styled(format!("/{}", spec.name), name_style),
                    Span::raw(" ".repeat(name_pad)),
                    Span::styled(spec.summary, desc_style),
                ])
            })
            .collect();

        Paragraph::new(lines).render(inner, buf);
    }
}

/// Input history ring buffer.
pub struct InputHistory {
    entries: Vec<String>,
    /// Current position in history (None = new input, Some(i) = browsing).
    cursor: Option<usize>,
    /// Saved current input when browsing history.
    saved_input: String,
    max_entries: usize,
}

impl Default for InputHistory {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            cursor: None,
            saved_input: String::new(),
            max_entries: 100,
        }
    }
}

impl InputHistory {
    /// Record a submitted prompt.
    pub fn push(&mut self, text: String) {
        // Don't record duplicates of the last entry
        if self.entries.last().map_or(true, |last| last != &text) {
            self.entries.push(text);
            if self.entries.len() > self.max_entries {
                self.entries.remove(0);
            }
        }
        self.cursor = None;
    }

    /// Move to the previous (older) entry. Returns the entry text to display.
    pub fn prev(&mut self, current_input: &str) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }
        match self.cursor {
            None => {
                self.saved_input = current_input.to_string();
                self.cursor = Some(self.entries.len() - 1);
            }
            Some(i) if i > 0 => {
                self.cursor = Some(i - 1);
            }
            _ => return None, // Already at oldest
        }
        self.cursor.map(|i| self.entries[i].as_str())
    }

    /// Move to the next (newer) entry. Returns the entry text or saved input.
    pub fn next(&mut self) -> Option<&str> {
        match self.cursor {
            Some(i) => {
                if i + 1 < self.entries.len() {
                    self.cursor = Some(i + 1);
                    Some(&self.entries[i + 1])
                } else {
                    self.cursor = None;
                    Some(&self.saved_input)
                }
            }
            None => None,
        }
    }

    /// Reset browsing position (when user types something new).
    pub fn reset_cursor(&mut self) {
        self.cursor = None;
    }
}
