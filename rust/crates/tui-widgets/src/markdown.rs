//! Streaming markdown renderer wrapping `tui-markdown`.
//!
//! Accumulates text deltas from the LLM stream and converts to Ratatui `Text`
//! using `tui-markdown`'s pulldown-cmark + syntect integration.

use ratatui::text::Text;

/// Accumulates streaming markdown deltas and renders to Ratatui `Text`.
///
/// Uses `tui-markdown` (maintained by the Ratatui core team, used by Codex CLI)
/// for the actual markdown→widget conversion, with syntax-highlighted code blocks
/// via the `highlight-code` feature.
#[derive(Debug, Default, Clone)]
pub struct MarkdownStream {
    /// Raw accumulated markdown text.
    raw: String,
}

impl MarkdownStream {
    /// Create a new empty stream.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a stream pre-filled with content.
    #[must_use]
    pub fn from_str(content: &str) -> Self {
        Self {
            raw: content.to_string(),
        }
    }

    /// Append a streaming text delta.
    pub fn push_delta(&mut self, delta: &str) {
        self.raw.push_str(delta);
    }

    /// Get the raw accumulated markdown.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Returns true if no content has been accumulated.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    /// Render the accumulated markdown to Ratatui `Text` with syntax highlighting.
    #[must_use]
    pub fn to_text(&self) -> Text<'_> {
        if self.raw.is_empty() {
            return Text::default();
        }
        tui_markdown::from_str(&self.raw)
    }

    /// Clear all accumulated content.
    pub fn clear(&mut self) {
        self.raw.clear();
    }

    /// Get the estimated line count for scroll calculations.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.raw.lines().count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_stream_produces_empty_text() {
        let stream = MarkdownStream::new();
        assert!(stream.is_empty());
    }

    #[test]
    fn accumulates_deltas() {
        let mut stream = MarkdownStream::new();
        stream.push_delta("# Hello");
        stream.push_delta(" World\n\nBody text");
        assert_eq!(stream.raw(), "# Hello World\n\nBody text");
    }

    #[test]
    fn from_str_works() {
        let stream = MarkdownStream::from_str("**bold** text");
        assert!(!stream.is_empty());
    }
}
