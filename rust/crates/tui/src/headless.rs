//! Headless JSON-RPC mode for the Ink TUI bridge.
//!
//! Instead of rendering ratatui frames, this module:
//! - Reads `UiEvent` from the event channel and writes JSON Lines to stdout
//! - Reads JSON Lines from stdin and sends `Action` to the action channel
//!
//! ## JSON Lines protocol
//!
//! Every message is a single JSON object on one line, terminated by `\n`.
//!
//! **stdout (engine -> Ink TUI):**
//! ```json
//! {"type":"stream_delta","agentId":"primary","text":"Hello"}
//! {"type":"tool_call_start","agentId":"primary","callId":"t1","toolName":"Bash","inputPreview":"ls"}
//! ```
//!
//! **stdin (Ink TUI -> engine):**
//! ```json
//! {"type":"submit_prompt","text":"Build a web app"}
//! {"type":"resolve_permission","requestId":"r1","decision":"allow"}
//! ```
//!
//! ## Design decisions
//!
//! - `UiEvent::Tick` is **skipped** — it's a 100ms animation timer used only by ratatui.
//!   The Ink TUI handles its own animation timers via React/Ink refresh cycles.
//! - **stdout** is exclusively for JSON protocol lines. All logs and errors go to **stderr**.
//! - stdout is flushed after every line for real-time streaming.
//! - stdin EOF or channel close triggers a clean shutdown.

use std::io::{self, BufRead, Write};

use events::{Action, ActionTx, UiEvent, UiEventRx};

/// Run the headless JSON-RPC bridge.
///
/// This replaces the ratatui TUI event loop. It reads `UiEvent`s from the
/// backend orchestrator and writes them as JSON Lines to stdout, while reading
/// `Action`s from stdin JSON Lines and forwarding them to the backend.
///
/// Returns when:
/// - The event channel is closed (engine shutting down)
/// - stdin is closed (Ink TUI process exited)
/// - An unrecoverable I/O error occurs
pub async fn run(mut event_rx: UiEventRx, action_tx: ActionTx) -> anyhow::Result<()> {
    // Spawn stdin reader on a dedicated blocking thread.
    // We can't use async stdin because std::io::Stdin::lock() is blocking
    // and tokio::io::stdin() on Windows doesn't support line-buffered reads well.
    let action_tx_clone = action_tx.clone();
    let stdin_handle = tokio::task::spawn_blocking(move || {
        let stdin = io::stdin();
        let reader = stdin.lock();
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    let trimmed = line.trim().to_string();
                    if trimmed.is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<Action>(&trimmed) {
                        Ok(action) => {
                            if action_tx_clone.blocking_send(action).is_err() {
                                // Action channel closed — engine is shutting down
                                eprintln!("[headless] Action channel closed, stopping stdin reader");
                                break;
                            }
                        }
                        Err(e) => {
                            // Write parse errors to stderr — stdout is reserved for protocol
                            eprintln!("[headless] JSON parse error: {e} — line: {trimmed}");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[headless] stdin read error: {e}");
                    break;
                }
            }
        }
        eprintln!("[headless] stdin reader exited");
    });

    // Event loop: read UiEvent from channel, serialize to JSON, write to stdout.
    // We hold the stdout lock for the entire loop to avoid re-acquiring it per line.
    let stdout = io::stdout();
    let mut writer = io::BufWriter::new(stdout.lock());

    loop {
        match event_rx.recv().await {
            Some(UiEvent::Tick) => {
                // Skip Tick events — they're animation-only, not needed in headless mode.
                // The Ink TUI handles its own animation timers.
                continue;
            }
            Some(event) => {
                match serde_json::to_string(&event) {
                    Ok(json) => {
                        if let Err(e) = writeln!(writer, "{json}") {
                            // stdout broken (pipe closed) — Ink TUI exited
                            eprintln!("[headless] stdout write error: {e}");
                            break;
                        }
                        if let Err(e) = writer.flush() {
                            eprintln!("[headless] stdout flush error: {e}");
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("[headless] serialize error for event: {e}");
                        // Continue — don't crash on a single bad event
                    }
                }
            }
            None => {
                // Event channel closed — engine is shutting down
                eprintln!("[headless] event channel closed, shutting down");
                break;
            }
        }
    }

    // Clean up the stdin reader thread
    stdin_handle.abort();
    let _ = stdin_handle.await;

    Ok(())
}
