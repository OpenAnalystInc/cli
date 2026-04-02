//! File tree widget wrapping `tui-tree-widget`.
//!
//! Provides a sidebar file browser with collapsible directories.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use tui_tree_widget::{Tree, TreeState};

/// Re-export `TreeItem` for callers to build tree data.
pub use tui_tree_widget::TreeItem;

/// File tree sidebar widget.
pub struct FileTree<'a> {
    items: Vec<TreeItem<'a, String>>,
}

impl<'a> FileTree<'a> {
    /// Create a file tree from a list of tree items.
    #[must_use]
    pub fn new(items: Vec<TreeItem<'a, String>>) -> Self {
        Self { items }
    }

    /// Render the file tree with state.
    pub fn render_with_state(self, area: Rect, buf: &mut Buffer, state: &mut FileTreeState) {
        let Ok(tree) = Tree::new(&self.items) else {
            return; // Gracefully skip rendering if tree data is invalid
        };
        let tree = tree.highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        ratatui::widgets::StatefulWidget::render(tree, area, buf, &mut state.inner);
    }
}

/// State for the file tree, wrapping `tui_tree_widget::TreeState`.
pub struct FileTreeState {
    pub inner: TreeState<String>,
}

impl Default for FileTreeState {
    fn default() -> Self {
        Self {
            inner: TreeState::default(),
        }
    }
}

impl FileTreeState {
    /// Toggle the currently selected node open/closed.
    pub fn toggle_selected(&mut self) {
        self.inner.toggle_selected();
    }

    /// Move selection up.
    pub fn select_previous(&mut self) {
        self.inner.key_up();
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        self.inner.key_down();
    }
}
