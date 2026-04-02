//! Dev preview — renders a single TUI frame with mock data to see the layout.
//!
//! Run with: cargo run -p tui --example dev_preview

use std::io;
use std::time::Duration;

use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::Terminal;

use events::{AgentStatus, AgentType};
use tui::app::App;
use tui::banner::BannerAccountInfo;
use tui::panels::sidebar::FileAction;
use tui_widgets::status_bar::AgentPhase;
use tui_widgets::{ToolCallCard, ToolCallStatus};

fn main() -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app with dummy channels
    let (ui_tx, ui_rx) = tokio::sync::mpsc::channel(1);
    let (action_tx, _action_rx) = tokio::sync::mpsc::channel(1);
    let mut app = App::new(ui_rx, action_tx, "openanalyst-beta");

    // Inject mock banner
    app.set_banner(BannerAccountInfo {
        display_name: "Anit".to_string(),
        model_display: "Opus 4.6 (1M context)".to_string(),
        provider_name: "Claude Max".to_string(),
        user_email: Some("tools@openanalyst.com".to_string()),
        organization: Some("Org".to_string()),
        cwd: "D:\\Openanalyst CLI".to_string(),
        version: "1.0.89".to_string(),
    });

    // Add mock messages
    app.chat.push_user("fix the login bug in auth.rs".to_string());

    app.chat.start_assistant();
    app.chat.push_delta("I'll look at the auth module and fix the login issue. Let me read the file first.");
    app.chat.finish_assistant();

    // Add tool call cards
    app.chat.push_tool_call(ToolCallCard {
        tool_name: "Read".to_string(),
        input_preview: "src/auth.rs".to_string(),
        status: ToolCallStatus::Completed {
            duration: Duration::from_millis(340),
        },
        output: Some("fn login(user: &str, pass: &str) -> Result<Token> {\n    let client = reqwest::Client::new();\n    // BUG: missing await on async call\n    let resp = client.post(AUTH_URL).json(&creds).send();\n    Ok(resp.json::<Token>()?)\n}".to_string()),
        expanded: false,
    });

    app.chat.push_tool_call(ToolCallCard {
        tool_name: "Edit".to_string(),
        input_preview: "src/auth.rs:42".to_string(),
        status: ToolCallStatus::Completed {
            duration: Duration::from_millis(1250),
        },
        output: Some("Applied edit: added .await to send() call".to_string()),
        expanded: true,
    });

    app.chat.start_assistant();
    app.chat.push_delta("Found the issue — the OAuth token wasn't being awaited properly. Fixed by adding `.await` to the `send()` call on line 42.");
    app.chat.finish_assistant();

    app.chat.push_user("now run the tests".to_string());

    app.chat.push_tool_call(ToolCallCard {
        tool_name: "Bash".to_string(),
        input_preview: "cargo test --lib auth".to_string(),
        status: ToolCallStatus::Running {
            elapsed: Duration::from_millis(4500),
        },
        output: None,
        expanded: false,
    });

    // Mock sidebar state
    app.sidebar_state.update_agent(
        "primary".to_string(),
        AgentType::Primary,
        "Fix login bug".to_string(),
        AgentStatus::Running,
    );
    app.sidebar_state.update_agent(
        "agent-1".to_string(),
        AgentType::Explore,
        "Search for auth tests".to_string(),
        AgentStatus::Completed,
    );

    app.sidebar_state.track_file("src/auth.rs".to_string(), FileAction::Edited);
    app.sidebar_state.track_file("src/main.rs".to_string(), FileAction::Read);
    app.sidebar_state.track_file("tests/auth_test.rs".to_string(), FileAction::Read);
    app.sidebar_state.track_file("Cargo.toml".to_string(), FileAction::Read);
    app.sidebar_state.tool_call_count = 3;

    // Set status bar state
    app.status_bar.phase = AgentPhase::RunningBash;
    app.status_bar.elapsed = Duration::from_secs(47);
    app.status_bar.total_tokens = 2_450;
    app.is_streaming = true;

    // Draw one frame
    terminal.draw(|frame| {
        app.render(frame.area(), frame.buffer_mut());
    })?;

    // Wait for any key press
    loop {
        if ratatui::crossterm::event::poll(Duration::from_millis(100))? {
            if let ratatui::crossterm::event::Event::Key(_) = ratatui::crossterm::event::read()? {
                break;
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    Ok(())
}
