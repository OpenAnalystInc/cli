//! Dev preview — renders a TUI frame to text output (no alternate screen).
//!
//! Run with: cargo run -p tui --example dev_preview_text

use std::time::Duration;

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use events::{AgentStatus, AgentType};
use tui::app::App;
use tui::banner::BannerAccountInfo;
use tui::panels::sidebar::FileAction;
use tui_widgets::status_bar::AgentPhase;
use tui_widgets::{ToolCallCard, ToolCallStatus};

fn main() {
    // Create a test backend (130x40 terminal)
    let backend = TestBackend::new(130, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    // Create app with dummy channels
    let (_ui_tx, ui_rx) = tokio::sync::mpsc::channel(1);
    let (action_tx, _action_rx) = tokio::sync::mpsc::channel(1);
    let mut app = App::new(ui_rx, action_tx, "openanalyst-beta");

    // Inject banner
    app.set_banner(BannerAccountInfo {
        display_name: "Anit".to_string(),
        model_display: "Opus 4.6 (1M context)".to_string(),
        provider_name: "Claude Max".to_string(),
        user_email: Some("tools@openanalyst.com".to_string()),
        organization: Some("Org".to_string()),
        cwd: "D:\\Openanalyst CLI".to_string(),
        version: "1.0.89".to_string(),
    });

    // Don't auto_scroll so banner stays visible
    app.chat.auto_scroll = false;
    app.chat.scroll_offset = 0;

    // Add mock conversation
    app.chat.push_user("fix the login bug in auth.rs".to_string());

    app.chat.start_assistant();
    app.chat.push_delta("I'll look at the auth module and fix the login issue.");
    app.chat.finish_assistant();

    // Tool call cards
    app.chat.push_tool_call(ToolCallCard {
        tool_name: "Read".to_string(),
        input_preview: "src/auth.rs".to_string(),
        status: ToolCallStatus::Completed {
            duration: Duration::from_millis(340),
        },
        output: Some("fn login() { ... }".to_string()),
        expanded: false, diff: None,
    });

    app.chat.push_tool_call(ToolCallCard {
        tool_name: "Edit".to_string(),
        input_preview: "src/auth.rs:42".to_string(),
        status: ToolCallStatus::Completed {
            duration: Duration::from_millis(1250),
        },
        output: Some("Applied: added .await to send() call".to_string()),
        expanded: true, diff: None,
    });

    app.chat.start_assistant();
    app.chat.push_delta("Fixed the async issue on line 42. All tests should pass now.");
    app.chat.finish_assistant();

    app.chat.push_tool_call(ToolCallCard {
        tool_name: "Bash".to_string(),
        input_preview: "cargo test --lib auth".to_string(),
        status: ToolCallStatus::Running {
            elapsed: Duration::from_millis(4500),
        },
        output: None,
        expanded: false, diff: None,
    });

    // Sidebar mock data
    app.sidebar_state.update_agent(
        "primary".to_string(),
        AgentType::Primary,
        "Fix login bug".to_string(),
        AgentStatus::Running,
    );
    app.sidebar_state.update_agent(
        "agent-1".to_string(),
        AgentType::Explore,
        "Search auth tests".to_string(),
        AgentStatus::Completed,
    );

    app.sidebar_state.track_file("src/auth.rs".to_string(), FileAction::Edited);
    app.sidebar_state.track_file("src/main.rs".to_string(), FileAction::Read);
    app.sidebar_state.track_file("tests/auth_test.rs".to_string(), FileAction::Read);
    app.sidebar_state.track_file("Cargo.toml".to_string(), FileAction::Read);
    app.sidebar_state.tool_call_count = 3;

    // Status bar
    app.status_bar.phase = AgentPhase::RunningBash;
    app.status_bar.elapsed = Duration::from_secs(47);
    app.status_bar.total_tokens = 2_450;
    app.is_streaming = true;

    // Keep scroll at top so banner is visible
    app.chat.auto_scroll = false;
    app.chat.scroll_offset = 0;

    // Render one frame
    terminal.draw(|frame| {
        app.render(frame.area(), frame.buffer_mut());
    }).unwrap();

    // Print the buffer contents
    let backend = terminal.backend();
    let buffer = backend.buffer();

    println!();
    println!("  OpenAnalyst CLI — TUI Dev Preview ({}x{})", buffer.area.width, buffer.area.height);
    println!("  ┌{}┐", "─".repeat(buffer.area.width as usize));
    for y in 0..buffer.area.height {
        let mut line = String::new();
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            line.push_str(cell.symbol());
        }
        let trimmed = line.trim_end();
        println!("  │{:<width$}│", trimmed, width = buffer.area.width as usize);
    }
    println!("  └{}┘", "─".repeat(buffer.area.width as usize));
    println!();
}
