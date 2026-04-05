//! Input box wrapping `edtui` for vim-mode text editing.
//!
//! `edtui` provides Normal/Insert/Visual modes, clipboard, undo/redo out of the box.
//! We wrap it with submit handling (Ctrl+S / Enter), mode-aware borders, and
//! dynamic height calculation.

use edtui::{EditorEventHandler, EditorMode, EditorState, EditorTheme, EditorView, Lines};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Widget};

/// The permission level that controls tool approval behavior.
/// This is the "mode" the user sees and cycles through with Ctrl+P.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionLevel {
    /// Default: ask before running tools.
    Default,
    /// Plan mode: read-only tools only, planning only.
    Plan,
    /// Accept Edits: auto-approve file write/edit, prompt for shell.
    AcceptEdits,
    /// Danger (Full Access): everything auto-approved.
    Danger,
}

impl PermissionLevel {
    /// Cycle to the next permission level.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Default => Self::Plan,
            Self::Plan => Self::AcceptEdits,
            Self::AcceptEdits => Self::Danger,
            Self::Danger => Self::Default,
        }
    }

    /// Display label for the mode indicator.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Plan => "Plan",
            Self::AcceptEdits => "Accept Edits",
            Self::Danger => "Danger",
        }
    }

    /// Map to the runtime permission mode string.
    #[must_use]
    pub fn to_permission_mode(self) -> &'static str {
        match self {
            Self::Default => "prompt",
            Self::Plan => "read-only",
            Self::AcceptEdits => "workspace-write",
            Self::Danger => "danger-full-access",
        }
    }

    fn accent_color(self) -> Color {
        match self {
            Self::Default => Color::Rgb(50, 130, 255), // blue
            Self::Plan => Color::Yellow,
            Self::AcceptEdits => Color::Green,
            Self::Danger => Color::Red,
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Default => "❯",
            Self::Plan => "◈",
            Self::AcceptEdits => "✎",
            Self::Danger => "⚡",
        }
    }
}

impl Default for PermissionLevel {
    fn default() -> Self {
        Self::Default
    }
}

/// The current activity state displayed in the input box title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    /// Ready for user input.
    Ready,
    /// An agent is running — show which one.
    AgentRunning { label: String },
    /// A plan is being executed.
    PlanRunning { label: String },
    /// Streaming response in progress.
    Streaming,
}

impl Default for InputMode {
    fn default() -> Self {
        Self::Ready
    }
}

impl InputMode {
    fn border_color(&self, perm: PermissionLevel) -> Color {
        match self {
            Self::Ready => perm.accent_color(),
            Self::AgentRunning { .. } => Color::Rgb(50, 130, 255),
            Self::PlanRunning { .. } => Color::Yellow,
            Self::Streaming => Color::Cyan,
        }
    }

    fn title_spans(&self, perm: PermissionLevel, _active_agent: Option<&str>) -> Vec<Span<'static>> {
        match self {
            Self::Ready => {
                // Clean left title: icon + hint text only
                // Mode badge, agent badge, and branch are on the right side
                vec![
                    Span::styled(
                        format!(" {} ", perm.icon()),
                        Style::default().fg(perm.accent_color()).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "Enter to send · Ctrl+P mode ",
                        Style::default().fg(Color::Indexed(240)),
                    ),
                ]
            }
            Self::AgentRunning { label } => vec![
                Span::styled(
                    " ● ",
                    Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{label} "),
                    Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "· Ctrl+C to cancel ",
                    Style::default().fg(Color::Indexed(240)),
                ),
            ],
            Self::PlanRunning { label } => vec![
                Span::styled(
                    " ◈ ",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{label} "),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "· Ctrl+C to cancel ",
                    Style::default().fg(Color::Indexed(240)),
                ),
            ],
            Self::Streaming => vec![
                Span::styled(
                    " ⠋ ",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Responding... ",
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    "· Ctrl+C to cancel ",
                    Style::default().fg(Color::Indexed(240)),
                ),
            ],
        }
    }
}

/// Wrapper around `edtui` providing a vim-mode input area with mode-aware styling.
pub struct InputBox {
    mode: InputMode,
    /// Current permission level (changes border color and mode indicator).
    permission_level: PermissionLevel,
    /// Right-aligned context tag (git branch, active plan, agent name).
    context_tag: Option<String>,
    /// Model label shown in the right-side badges (e.g., "opus-4-6").
    model_label: Option<String>,
    /// Active agent name (from sidebar selection).
    active_agent: Option<String>,
    /// Context files attached from sidebar (shown as @file badges).
    context_files: Vec<String>,
    /// Credit/balance status (shown bottom-right, e.g., "free tier" or "42 credits").
    credit_status: Option<String>,
    /// Connected MCP servers count (shown bottom-right if > 0).
    mcp_count: usize,
}

impl Default for InputBox {
    fn default() -> Self {
        Self {
            mode: InputMode::Ready,
            permission_level: PermissionLevel::Default,
            context_tag: None,
            model_label: None,
            active_agent: None,
            context_files: Vec::new(),
            credit_status: None,
            mcp_count: 0,
        }
    }
}

impl InputBox {
    /// Set the input mode (changes border color and title).
    #[must_use]
    pub fn mode(mut self, mode: InputMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set the permission level (changes border color and mode indicator).
    #[must_use]
    pub fn permission_level(mut self, level: PermissionLevel) -> Self {
        self.permission_level = level;
        self
    }

    /// Set the context tag (displayed top-right of the input border).
    #[must_use]
    pub fn context_tag(mut self, tag: Option<String>) -> Self {
        self.context_tag = tag;
        self
    }

    /// Set the model label (displayed as right-side badge, e.g., "opus-4-6").
    #[must_use]
    pub fn model_label(mut self, label: Option<String>) -> Self {
        self.model_label = label;
        self
    }

    /// Set the active agent name (displayed in title when an agent is selected).
    #[must_use]
    pub fn active_agent(mut self, name: Option<String>) -> Self {
        self.active_agent = name;
        self
    }

    /// Set context files (shown as @file badges in the bottom border).
    #[must_use]
    pub fn context_files(mut self, files: Vec<String>) -> Self {
        self.context_files = files;
        self
    }

    /// Set credit/balance status (shown bottom-right).
    #[must_use]
    pub fn credit_status(mut self, status: Option<String>) -> Self {
        self.credit_status = status;
        self
    }

    /// Set connected MCP server count (shown bottom-right if > 0).
    #[must_use]
    pub fn mcp_count(mut self, count: usize) -> Self {
        self.mcp_count = count;
        self
    }

    /// Render the input box into the given area using the provided state.
    pub fn render_with_state(self, area: Rect, buf: &mut Buffer, state: &mut InputBoxState) {
        let border_color = self.mode.border_color(self.permission_level);
        let title_spans = self.mode.title_spans(self.permission_level, self.active_agent.as_deref());

        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .border_style(Style::default().fg(border_color))
            .title(Line::from(title_spans));

        // Right-aligned badges: [mode] [agent] [branch] — like Claude Code
        {
            let mut right_spans: Vec<Span<'static>> = Vec::new();

            // Permission mode badge (only when not Default)
            if self.permission_level != PermissionLevel::Default {
                right_spans.push(Span::styled(
                    format!(" {} {} ", self.permission_level.icon(), self.permission_level.label()),
                    Style::default()
                        .fg(Color::Black)
                        .bg(self.permission_level.accent_color())
                        .add_modifier(Modifier::BOLD),
                ));
                right_spans.push(Span::styled(" ", Style::default()));
            }

            // Model label badge (like Claude Code shows the model)
            if let Some(ref label) = self.model_label {
                right_spans.push(Span::styled(
                    format!(" {label} "),
                    Style::default()
                        .fg(Color::Indexed(252))
                        .bg(Color::Indexed(238))
                        .add_modifier(Modifier::BOLD),
                ));
                right_spans.push(Span::styled(" ", Style::default()));
            }

            // Active agent badge
            if let Some(ref agent_name) = self.active_agent {
                right_spans.push(Span::styled(
                    format!(" {agent_name} "),
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Rgb(180, 120, 255)) // purple for agent
                        .add_modifier(Modifier::BOLD),
                ));
                right_spans.push(Span::styled(" ", Style::default()));
            }

            // Git branch badge
            if let Some(ref tag) = self.context_tag {
                right_spans.push(Span::styled(
                    format!(" {tag} "),
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Rgb(50, 130, 255))
                        .add_modifier(Modifier::BOLD),
                ));
            }

            if !right_spans.is_empty() {
                block = block.title_top(
                    Line::from(right_spans)
                        .alignment(ratatui::layout::Alignment::Right),
                );
            }
        }

        // Context file badges (bottom border)
        if !self.context_files.is_empty() {
            let max_width = area.width.saturating_sub(4) as usize;
            let mut file_spans: Vec<Span<'static>> = Vec::new();
            let mut used_width = 0usize;
            let total = self.context_files.len();
            let mut shown = 0usize;

            for path in &self.context_files {
                let short = path.rsplit(['/', '\\']).next().unwrap_or(path);
                let badge = format!(" @{short} ");
                let badge_len = badge.len() + 1; // +1 for separator space
                if used_width + badge_len > max_width && shown > 0 {
                    let remaining = total - shown;
                    file_spans.push(Span::styled(
                        format!(" +{remaining} more "),
                        Style::default().fg(Color::Indexed(245)),
                    ));
                    break;
                }
                file_spans.push(Span::styled(
                    badge,
                    Style::default().fg(Color::Cyan).bg(Color::Indexed(236)),
                ));
                file_spans.push(Span::styled(" ", Style::default()));
                used_width += badge_len;
                shown += 1;
            }

            block = block.title_bottom(Line::from(file_spans));
        }

        // Bottom-right: credit status + MCP count
        {
            let mut bottom_right: Vec<Span<'static>> = Vec::new();
            if let Some(ref status) = self.credit_status {
                bottom_right.push(Span::styled(
                    format!(" {status} "),
                    Style::default().fg(Color::Indexed(245)).bg(Color::Indexed(236)),
                ));
                bottom_right.push(Span::styled(" ", Style::default()));
            }
            if self.mcp_count > 0 {
                bottom_right.push(Span::styled(
                    format!(" MCP:{} ", self.mcp_count),
                    Style::default().fg(Color::Green).bg(Color::Indexed(236)),
                ));
            }
            if !bottom_right.is_empty() {
                block = block.title_bottom(
                    Line::from(bottom_right)
                        .alignment(ratatui::layout::Alignment::Right),
                );
            }
        }

        let inner = block.inner(area);
        block.render(area, buf);

        let theme = EditorTheme {
            base: Style::default().fg(Color::Indexed(252)),
            cursor_style: Style::default().fg(Color::Black).bg(Color::White),
            selection_style: Style::default().bg(Color::Indexed(236)),
            status_line: None,
            block: None,
            line_numbers_style: Style::default().fg(Color::Indexed(238)),
        };

        // Clip by 1 row to hide edtui's built-in vim status line
        let clipped = if inner.height > 1 {
            Rect { height: inner.height - 1, ..inner }
        } else {
            inner
        };

        let editor = EditorView::new(&mut state.editor).theme(theme);
        editor.render(clipped, buf);
    }
}

/// State for the input box, wrapping `edtui::EditorState` and `EditorEventHandler`.
pub struct InputBoxState {
    pub editor: EditorState,
    pub event_handler: EditorEventHandler,
}

impl Default for InputBoxState {
    fn default() -> Self {
        let mut editor = EditorState::default();
        // Start in Insert mode so users can type immediately (not vim Normal mode)
        editor.mode = EditorMode::Insert;
        Self {
            editor,
            event_handler: EditorEventHandler::default(),
        }
    }
}

impl InputBoxState {
    /// Create with vim keybindings (Normal/Insert/Visual modes).
    #[must_use]
    pub fn with_vim_mode() -> Self {
        let mut editor = EditorState::default();
        editor.mode = EditorMode::Insert;
        Self {
            editor,
            event_handler: EditorEventHandler::vim_mode(),
        }
    }

    /// Get the current text content.
    #[must_use]
    pub fn text(&self) -> String {
        String::from(self.editor.lines.clone())
    }

    /// Set the input text (replaces current content).
    pub fn set_text(&mut self, text: &str) {
        self.editor = EditorState::new(Lines::from(text));
        // Stay in Insert mode after setting text
        self.editor.mode = EditorMode::Insert;
    }

    /// Clear the input.
    pub fn clear(&mut self) {
        self.editor = EditorState::new(Lines::default());
        // Stay in Insert mode after clearing
        self.editor.mode = EditorMode::Insert;
    }

    /// Get the number of lines in the current editor content.
    /// Used by the layout to dynamically size the input area.
    #[must_use]
    pub fn line_count(&self) -> u16 {
        let text = self.text();
        let lines = text.lines().count().max(1) as u16;
        // +2 for border top/bottom
        lines + 2
    }

    /// Handle a key event. Returns `Some(text)` if the user submitted.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<String> {
        // Ctrl+S → submit (always)
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
            let text = self.text();
            if !text.trim().is_empty() {
                self.clear();
                return Some(text);
            }
            return None;
        }

        // Ctrl+V → paste from system clipboard (text or image path)
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('v') {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                // Try text first
                if let Ok(text) = clipboard.get_text() {
                    if !text.is_empty() {
                        let current = self.text();
                        self.set_text(&format!("{current}{text}"));
                        return None;
                    }
                }
                // Try image — save to temp file and insert path reference
                if let Ok(image) = clipboard.get_image() {
                    let temp_dir = std::env::temp_dir().join("openanalyst-clipboard");
                    let _ = std::fs::create_dir_all(&temp_dir);
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis();
                    let img_path = temp_dir.join(format!("paste-{ts}.png"));
                    // Convert RGBA bytes to PNG
                    if let Ok(()) = save_rgba_as_png(
                        &image.bytes,
                        image.width as u32,
                        image.height as u32,
                        &img_path,
                    ) {
                        let current = self.text();
                        let path_str = img_path.display().to_string();
                        self.set_text(&format!("{current}[image] {path_str}"));
                    }
                }
            }
            return None;
        }

        // Ctrl+Z → undo (delegates to edtui's undo)
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('z') {
            // Send 'u' key to edtui in Normal mode for undo
            let prev_mode = self.editor.mode;
            self.editor.mode = EditorMode::Normal;
            self.event_handler.on_key_event(
                KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE),
                &mut self.editor,
            );
            self.editor.mode = prev_mode;
            return None;
        }

        // Enter → submit (works for single-line and multiline/pasted text)
        if key.code == KeyCode::Enter && !key.modifiers.contains(KeyModifiers::SHIFT) {
            let text = self.text();
            if !text.trim().is_empty() {
                self.clear();
                return Some(text);
            }
            // Empty input: swallow the Enter (don't insert a newline)
            return None;
        }

        // Delegate to edtui vim-mode handler
        self.event_handler.on_key_event(key, &mut self.editor);
        None
    }
}

/// Save raw RGBA bytes as a PNG file (minimal implementation, no extra deps).
fn save_rgba_as_png(rgba: &[u8], width: u32, height: u32, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let mut file = std::fs::File::create(path)?;

    // BMP is simpler than PNG and doesn't need a compression library
    // Write as BMP (BITMAPFILEHEADER + BITMAPINFOHEADER + pixel data)
    let row_size = (width * 3 + 3) & !3; // rows padded to 4-byte boundary
    let pixel_data_size = row_size * height;
    let file_size = 54 + pixel_data_size;

    // BITMAPFILEHEADER (14 bytes)
    file.write_all(b"BM")?;
    file.write_all(&(file_size).to_le_bytes())?;
    file.write_all(&0u16.to_le_bytes())?; // reserved
    file.write_all(&0u16.to_le_bytes())?; // reserved
    file.write_all(&54u32.to_le_bytes())?; // pixel data offset

    // BITMAPINFOHEADER (40 bytes)
    file.write_all(&40u32.to_le_bytes())?;
    file.write_all(&(width).to_le_bytes())?;
    file.write_all(&(height).to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?; // planes
    file.write_all(&24u16.to_le_bytes())?; // bits per pixel (BGR)
    file.write_all(&0u32.to_le_bytes())?; // compression (none)
    file.write_all(&(pixel_data_size).to_le_bytes())?;
    file.write_all(&2835u32.to_le_bytes())?; // h resolution
    file.write_all(&2835u32.to_le_bytes())?; // v resolution
    file.write_all(&0u32.to_le_bytes())?; // colors
    file.write_all(&0u32.to_le_bytes())?; // important colors

    // Pixel data (BMP is bottom-up, BGR format)
    let mut row_buf = vec![0u8; row_size as usize];
    for y in (0..height).rev() {
        for x in 0..width {
            let src = ((y * width + x) * 4) as usize;
            let dst = (x * 3) as usize;
            if src + 2 < rgba.len() {
                row_buf[dst] = rgba[src + 2];     // B
                row_buf[dst + 1] = rgba[src + 1]; // G
                row_buf[dst + 2] = rgba[src];     // R
            }
        }
        file.write_all(&row_buf)?;
    }

    // Rename to .bmp since we're writing BMP format
    let bmp_path = path.with_extension("bmp");
    if path != bmp_path {
        let _ = std::fs::rename(path, &bmp_path);
    }

    Ok(())
}
