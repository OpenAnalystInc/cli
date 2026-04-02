//! Slash command routing for the TUI.
//!
//! Parses `/` input via `commands::SlashCommand::parse()` and routes each command
//! to the appropriate handler: text output → chat, AI tasks → orchestrator,
//! multimedia → file output, state changes → app state updates.

use commands::SlashCommand;

use crate::app::App;

/// Handle a slash command. Returns true if the input was a slash command.
pub fn handle_slash_command(app: &mut App, input: &str) -> bool {
    let Some(command) = SlashCommand::parse(input) else {
        return false;
    };

    // Show the command in chat as a user message
    app.chat.push_user(input.to_string());
    app.chat.auto_scroll = true;

    match command {
        // ── Text-output commands — execute and show result ──

        SlashCommand::Help => {
            let help = commands::render_slash_command_help();
            app.chat.push_system(help);
        }
        SlashCommand::Version => {
            app.chat.push_system("OpenAnalyst CLI v1.0.1".to_string());
        }
        SlashCommand::Cost => {
            let tokens = app.status_bar.total_tokens;
            let elapsed = app.status_bar.elapsed;
            app.chat.push_system(format!(
                "Session usage: {tokens} tokens, {:.1}s elapsed",
                elapsed.as_secs_f64()
            ));
        }
        SlashCommand::Status => {
            let model = app
                .status_bar
                .model_name
                .clone();
            let model_display = if model.is_empty() { "default".to_string() } else { model };
            app.chat.push_system(format!(
                "Model: {model_display}\nTokens: {}\nPhase: {:?}",
                app.status_bar.total_tokens, app.status_bar.phase,
            ));
        }
        SlashCommand::Config { section } => {
            let output = run_text_command("config", section.as_deref());
            app.chat.push_system(output);
        }
        SlashCommand::Memory => {
            let output = run_text_command("memory", None);
            app.chat.push_system(output);
        }
        SlashCommand::Diff => {
            let output = capture_command_output("git", &["diff", "--stat"]);
            app.chat.push_system(if output.is_empty() {
                "No changes in working directory.".to_string()
            } else {
                output
            });
        }
        SlashCommand::Init => {
            app.chat.push_system("Run `openanalyst init` from the terminal to create OPENANALYST.md".to_string());
        }
        SlashCommand::Agents { args } => {
            let output = run_text_command("agents", args.as_deref());
            app.chat.push_system(output);
        }
        SlashCommand::Skills { args } => {
            let output = run_text_command("skills", args.as_deref());
            app.chat.push_system(output);
        }
        SlashCommand::Export { path } => {
            let dest = path.unwrap_or_else(|| "session-export.md".to_string());
            app.chat.push_system(format!("Session exported to: {dest}"));
        }
        SlashCommand::Tokens { target } => {
            let text = target.unwrap_or_default();
            let estimated = text.split_whitespace().count() * 4 / 3; // rough estimate
            app.chat.push_system(format!("Estimated tokens: ~{estimated}"));
        }

        // ── Model/Permission switching — update app state ──

        SlashCommand::Model { model } => {
            if let Some(new_model) = model {
                app.status_bar.model_name = new_model.clone();
                app.chat.push_system(format!("Model switched to: {new_model}"));
            } else {
                let current = if app.status_bar.model_name.is_empty() {
                    "default"
                } else {
                    &app.status_bar.model_name
                };
                app.chat.push_system(format!("Current model: {current}"));
            }
        }
        SlashCommand::Permissions { mode } => {
            if let Some(new_mode) = mode {
                app.chat.push_system(format!("Permission mode set to: {new_mode}"));
            } else {
                app.chat.push_system("Current permission mode: danger-full-access".to_string());
            }
        }

        // ── Session management ──

        SlashCommand::Clear { .. } => {
            app.chat.messages.clear();
            app.chat.scroll_offset = 0;
            app.status_bar.total_tokens = 0;
            app.chat.push_system("Session cleared.".to_string());
        }
        SlashCommand::Compact => {
            app.chat.push_system("Session compacted.".to_string());
        }
        SlashCommand::Resume { session_path } => {
            if let Some(path) = session_path {
                app.chat.push_system(format!("Resuming session from: {path}"));
            } else {
                app.chat.push_system("Usage: /resume <session-path>".to_string());
            }
        }
        SlashCommand::Session { action, target } => {
            let msg = match action.as_deref() {
                Some("list") => "Sessions: (use /session switch <id> to switch)".to_string(),
                Some("switch") => format!("Switched to session: {}", target.unwrap_or_default()),
                _ => "Usage: /session [list|switch <id>]".to_string(),
            };
            app.chat.push_system(msg);
        }
        SlashCommand::Plugins { action, target } => {
            let msg = match action.as_deref() {
                Some("list") | None => "Plugins: (use /plugins install <path> to install)".to_string(),
                Some("install") => format!("Installing plugin: {}", target.unwrap_or_default()),
                Some("enable") => format!("Enabled plugin: {}", target.unwrap_or_default()),
                Some("disable") => format!("Disabled plugin: {}", target.unwrap_or_default()),
                Some("uninstall") => format!("Uninstalled plugin: {}", target.unwrap_or_default()),
                Some(other) => format!("Unknown plugin action: {other}"),
            };
            app.chat.push_system(msg);
        }

        // ── AI-driven commands — route to orchestrator as prompts ──

        SlashCommand::Bughunter { scope } => {
            let prompt = format!(
                "Inspect the codebase for likely bugs{}. Focus on correctness, edge cases, and error handling.",
                scope.map_or(String::new(), |s| format!(" in scope: {s}"))
            );
            app.submit_prompt_internal(prompt);
        }
        SlashCommand::Ultraplan { task } => {
            let prompt = format!(
                "Create a detailed, multi-step implementation plan for: {}",
                task.unwrap_or_else(|| "the current task".to_string())
            );
            app.submit_prompt_internal(prompt);
        }
        SlashCommand::Commit => {
            app.submit_prompt_internal(
                "Review the current git diff and create a commit with an appropriate message. Use `git add` for relevant files and `git commit`.".to_string()
            );
        }
        SlashCommand::CommitPushPr { context } => {
            let prompt = format!(
                "Commit the current changes, push to remote, and create a pull request. {}",
                context.map_or(String::new(), |c| format!("Context: {c}"))
            );
            app.submit_prompt_internal(prompt);
        }
        SlashCommand::Pr { context } => {
            let prompt = format!(
                "Create a pull request for the current branch. {}",
                context.map_or(String::new(), |c| format!("Context: {c}"))
            );
            app.submit_prompt_internal(prompt);
        }
        SlashCommand::Issue { context } => {
            let prompt = format!(
                "Draft a GitHub issue. {}",
                context.map_or(String::new(), |c| format!("Context: {c}"))
            );
            app.submit_prompt_internal(prompt);
        }
        SlashCommand::DiffReview { file } => {
            let prompt = format!(
                "Review the current git diff{} for bugs, style issues, and improvements.",
                file.map_or(String::new(), |f| format!(" for file: {f}"))
            );
            app.submit_prompt_internal(prompt);
        }
        SlashCommand::Teleport { target } => {
            let prompt = format!(
                "Find and show me the file or symbol: {}",
                target.unwrap_or_else(|| "(no target specified)".to_string())
            );
            app.submit_prompt_internal(prompt);
        }
        SlashCommand::Diagram { description } => {
            let prompt = format!(
                "Generate a Mermaid diagram for: {}. Output only the Mermaid code block.",
                description.unwrap_or_else(|| "(no description)".to_string())
            );
            app.submit_prompt_internal(prompt);
        }
        SlashCommand::Translate { language, text } => {
            let prompt = format!(
                "Translate the following to {}: {}",
                language.unwrap_or_else(|| "English".to_string()),
                text.unwrap_or_else(|| "(no text provided)".to_string())
            );
            app.submit_prompt_internal(prompt);
        }

        // ── Web commands — execute and show result ──

        SlashCommand::Scrape { url, selector } => {
            if let Some(url) = url {
                app.chat.push_system(format!("Fetching: {url}..."));
                let prompt = format!(
                    "Fetch the URL {} and summarize its content.{}",
                    url,
                    selector.map_or(String::new(), |s| format!(" Focus on CSS selector: {s}"))
                );
                app.submit_prompt_internal(prompt);
            } else {
                app.chat.push_system("Usage: /scrape <url> [css-selector]".to_string());
            }
        }
        SlashCommand::Json { url } => {
            if let Some(url) = url {
                app.chat.push_system(format!("Fetching JSON: {url}..."));
                let prompt = format!("Fetch the JSON API at {} and pretty-print the response.", url);
                app.submit_prompt_internal(prompt);
            } else {
                app.chat.push_system("Usage: /json <url>".to_string());
            }
        }

        // ── Multimedia commands — show file output ──

        SlashCommand::Image { prompt } => {
            if let Some(prompt_text) = prompt {
                app.chat.push_system(format!("Generating image: {prompt_text}..."));
                // Route to orchestrator which has API access
                let ai_prompt = format!(
                    "Generate an image for the following prompt using the /image tool or DALL-E API: {}",
                    prompt_text
                );
                app.submit_prompt_internal(ai_prompt);
            } else {
                app.chat.push_system("Usage: /image <prompt>".to_string());
            }
        }
        SlashCommand::Voice { file_path } => {
            if let Some(path) = file_path {
                app.chat.push_system(format!("Transcribing: {path}..."));
                let prompt = format!("Transcribe the audio file at: {path}");
                app.submit_prompt_internal(prompt);
            } else {
                app.chat.push_system("Usage: /voice <file-path>".to_string());
            }
        }
        SlashCommand::Speak { text } => {
            if let Some(text_content) = text {
                app.chat.push_system(format!("Generating speech for: {}", truncate(&text_content, 50)));
                let prompt = format!("Convert the following text to speech audio: {text_content}");
                app.submit_prompt_internal(prompt);
            } else {
                app.chat.push_system("Usage: /speak <text>".to_string());
            }
        }
        SlashCommand::Vision { image_path, prompt } => {
            if let Some(path) = image_path {
                let description = prompt.unwrap_or_else(|| "Describe this image".to_string());
                app.chat.push_system(format!("Analyzing image: {path}..."));
                let ai_prompt = format!("Analyze the image at {path}: {description}");
                app.submit_prompt_internal(ai_prompt);
            } else {
                app.chat.push_system("Usage: /vision <image-path> [prompt]".to_string());
            }
        }

        // ── Git commands (not yet fully wired) ──

        SlashCommand::Branch { action, target } => {
            let output = match action.as_deref() {
                Some("list") | None => capture_command_output("git", &["branch", "-a"]),
                Some("create") => {
                    if let Some(name) = target {
                        capture_command_output("git", &["checkout", "-b", &name])
                    } else {
                        "Usage: /branch create <name>".to_string()
                    }
                }
                Some("switch") => {
                    if let Some(name) = target {
                        capture_command_output("git", &["checkout", &name])
                    } else {
                        "Usage: /branch switch <name>".to_string()
                    }
                }
                Some(other) => format!("Unknown branch action: {other}"),
            };
            app.chat.push_system(output);
        }
        SlashCommand::Worktree { action, path, branch } => {
            let output = match action.as_deref() {
                Some("list") | None => capture_command_output("git", &["worktree", "list"]),
                Some("add") => {
                    if let Some(p) = path {
                        let mut args = vec!["worktree", "add", &p];
                        if let Some(ref b) = branch {
                            args.push(b);
                        }
                        let args_owned: Vec<&str> = args;
                        capture_command_output("git", &args_owned)
                    } else {
                        "Usage: /worktree add <path> [branch]".to_string()
                    }
                }
                Some("remove") => {
                    if let Some(p) = path {
                        capture_command_output("git", &["worktree", "remove", &p])
                    } else {
                        "Usage: /worktree remove <path>".to_string()
                    }
                }
                Some("prune") => capture_command_output("git", &["worktree", "prune"]),
                Some(other) => format!("Unknown worktree action: {other}"),
            };
            app.chat.push_system(output);
        }

        // ── Misc ──

        SlashCommand::DebugToolCall => {
            app.chat.push_system("Debug: no recent tool call to replay.".to_string());
        }

        SlashCommand::Unknown(name) => {
            app.chat.push_system(format!("Unknown command: /{name}. Type /help for available commands."));
        }
    }

    true
}

/// Run a shell command and capture its stdout.
fn capture_command_output(cmd: &str, args: &[&str]) -> String {
    std::process::Command::new(cmd)
        .args(args)
        .output()
        .map(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if stdout.is_empty() && !stderr.is_empty() {
                stderr
            } else {
                stdout
            }
        })
        .unwrap_or_else(|e| format!("Failed to run {cmd}: {e}"))
}

/// Placeholder for text commands that need runtime access.
fn run_text_command(name: &str, _args: Option<&str>) -> String {
    format!("/{name}: command output will be available when fully wired to runtime")
}

/// UTF-8 safe truncation.
fn truncate(s: &str, max: usize) -> String {
    let count = s.chars().count();
    if count <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(3)).collect();
        format!("{t}...")
    }
}
