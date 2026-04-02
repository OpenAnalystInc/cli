//! Slash command routing for the TUI.
//!
//! Parses `/` input via `commands::SlashCommand::parse()` and routes each
//! command to the appropriate handler: text output to chat, AI tasks to
//! orchestrator, multimedia to file output, state changes to app state.

use commands::SlashCommand;
use events::Action;

use crate::app::App;
use crate::panels::chat::ChatMessage;

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
            app.chat.push_system("OpenAnalyst CLI v1.0.89".to_string());
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
            let output = build_config_output(section.as_deref());
            app.chat.push_system(output);
        }
        SlashCommand::Memory => {
            let output = build_memory_output();
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
            let output = run_init();
            app.chat.push_system(output);
        }
        SlashCommand::Agents { args } => {
            let cwd = std::env::current_dir().unwrap_or_default();
            let output = commands::handle_agents_slash_command(args.as_deref(), &cwd)
                .unwrap_or_else(|e| format!("Error: {e}"));
            app.chat.push_system(output);
        }
        SlashCommand::Skills { args } => {
            let cwd = std::env::current_dir().unwrap_or_default();
            let output = commands::handle_skills_slash_command(args.as_deref(), &cwd)
                .unwrap_or_else(|e| format!("Error: {e}"));
            app.chat.push_system(output);
        }
        SlashCommand::Export { path } => {
            let dest = path.unwrap_or_else(|| "session-export.md".to_string());
            let output = export_session(&app.chat.messages, &dest);
            app.chat.push_system(output);
        }
        SlashCommand::Tokens { target } => {
            let text = target.unwrap_or_default();
            let estimated = text.split_whitespace().count() * 4 / 3; // rough estimate
            app.chat.push_system(format!("Estimated tokens: ~{estimated}"));
        }

        // ── Model/Permission switching — update app state ──

        SlashCommand::Model { model } => {
            if let Some(new_model) = model {
                // Validate: can we build a ProviderClient for this model?
                match api::ProviderClient::from_model(&new_model) {
                    Ok(_) => {
                        // Update display
                        app.status_bar.model_name = new_model.clone();
                        // Update the router so future prompts use the new model
                        app.router = orchestrator::router::ModelRouter::from_default_model(&new_model);
                        // Notify orchestrator to update its config
                        let tx = app.action_tx.clone();
                        let m = new_model.clone();
                        tokio::spawn(async move {
                            let _ = tx.send(Action::UpdateModel(m)).await;
                        });
                        let table = app.router.render_table();
                        app.chat.push_system(format!(
                            "Model switched to: {new_model}\nRouting table updated:\n{table}"
                        ));
                    }
                    Err(e) => {
                        app.chat.push_system(format!(
                            "Cannot switch to {new_model}: {e}\n\
                             Check that the provider's API key is set in your environment."
                        ));
                    }
                }
            } else {
                let current = if app.status_bar.model_name.is_empty() {
                    "default"
                } else {
                    &app.status_bar.model_name
                };
                let table = app.router.render_table();
                app.chat.push_system(format!("Current model: {current}\n{table}"));
            }
        }
        SlashCommand::Permissions { mode } => {
            if let Some(new_mode) = mode {
                // Validate the mode before accepting
                let valid = matches!(
                    new_mode.as_str(),
                    "read-only" | "readonly" | "ro"
                        | "workspace" | "workspace-write" | "ws"
                        | "full" | "danger-full-access" | "yolo"
                        | "prompt" | "ask" | "default"
                        | "allow" | "allow-all"
                );
                if valid {
                    app.permission_mode = new_mode.clone();
                    // Notify orchestrator
                    let tx = app.action_tx.clone();
                    let m = new_mode.clone();
                    tokio::spawn(async move {
                        let _ = tx.send(Action::UpdatePermissions(m)).await;
                    });
                    app.chat.push_system(format!("Permission mode set to: {new_mode}"));
                } else {
                    app.chat.push_system(format!(
                        "Unknown mode: {new_mode}\n\
                         Options: read-only, workspace, prompt (default), full, allow"
                    ));
                }
            } else {
                app.chat.push_system(format!(
                    "Current permission mode: {}\n\
                     Options: /permissions <read-only|workspace|prompt|full|allow>",
                    app.permission_mode
                ));
            }
        }

        // ── Session management ──

        SlashCommand::Clear { .. } => {
            app.chat.messages.clear();
            app.chat.scroll_offset = 0;
            app.chat.focused_message = None;
            app.status_bar.total_tokens = 0;
            app.chat.push_system("Session cleared.".to_string());
        }
        SlashCommand::Compact => {
            let before = app.chat.messages.len();
            compact_chat_messages(app);
            let after = app.chat.messages.len();
            let removed = before.saturating_sub(after);
            app.chat.push_system(format!(
                "Session compacted: {removed} messages removed, {after} kept."
            ));
        }
        SlashCommand::Resume { session_path } => {
            if let Some(path) = session_path {
                match load_session_into_chat(app, &path) {
                    Ok(count) => app.chat.push_system(format!("Resumed session from {path} ({count} messages)")),
                    Err(e) => app.chat.push_system(format!("Failed to load session: {e}")),
                }
            } else {
                app.chat.push_system("Usage: /resume <session-path>".to_string());
            }
        }
        SlashCommand::Session { action, target } => {
            let msg = match action.as_deref() {
                Some("list") => list_sessions(),
                Some("switch") => {
                    if let Some(id) = target {
                        match load_session_into_chat(app, &id) {
                            Ok(count) => format!("Switched to session ({count} messages)"),
                            Err(e) => format!("Failed to switch: {e}"),
                        }
                    } else {
                        "Usage: /session switch <session-id-or-path>".to_string()
                    }
                }
                _ => "Usage: /session [list|switch <id>]".to_string(),
            };
            app.chat.push_system(msg);
        }
        SlashCommand::Plugins { action, target } => {
            let msg = handle_plugins_command(action.as_deref(), target.as_deref());
            app.chat.push_system(msg);
        }

        // ── AI-driven commands — route to orchestrator as prompts ──

        SlashCommand::Bughunter { scope } => {
            let prompt = format!(
                "You are a senior security and reliability engineer. Systematically inspect the codebase{} for:\n\
                 1. Logic bugs — off-by-one, incorrect conditionals, wrong operator precedence\n\
                 2. Error handling gaps — unwrap on fallible ops, swallowed errors, missing edge cases\n\
                 3. Security issues — injection, XSS, path traversal, hardcoded secrets\n\
                 4. Concurrency bugs — data races, deadlocks, missing synchronization\n\
                 5. Resource leaks — unclosed files/connections, missing cleanup\n\n\
                 For each bug: state file, line, severity (critical/high/medium/low), root cause, and fix.",
                scope.map_or(String::new(), |s| format!(" in scope: {s}"))
            );
            app.submit_prompt_internal(prompt);
        }
        SlashCommand::Ultraplan { task } => {
            let prompt = format!(
                "Create a comprehensive implementation plan for: {}\n\n\
                 Structure your plan as:\n\
                 1. **Goal** — one sentence summary of the outcome\n\
                 2. **Constraints** — what must be preserved, what can't break\n\
                 3. **Architecture** — key files, modules, data flow changes\n\
                 4. **Steps** — ordered list of atomic, testable implementation steps\n\
                 5. **Risks** — what could go wrong, mitigation for each\n\
                 6. **Verification** — how to confirm each step succeeded",
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
                "You are a senior code reviewer. Review the current git diff{} with a critical eye.\n\
                 Check for: bugs, security issues, performance, error handling, API contract breaks.\n\
                 For each issue: state file, line, severity, and fix. If clean, say LGTM with summary.",
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

        // ── Git commands ──

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
            // Find the most recent tool call in chat history
            let last_tool = app.chat.messages.iter().rev().find_map(|msg| {
                if let ChatMessage::ToolCall { card } = msg {
                    Some(card.clone())
                } else {
                    None
                }
            });
            if let Some(card) = last_tool {
                let status = match &card.status {
                    tui_widgets::ToolCallStatus::Running { elapsed } => format!("Running ({:.1}s)", elapsed.as_secs_f64()),
                    tui_widgets::ToolCallStatus::Completed { duration } => format!("Completed ({:.1}s)", duration.as_secs_f64()),
                    tui_widgets::ToolCallStatus::Failed { duration } => format!("Failed ({:.1}s)", duration.as_secs_f64()),
                };
                let output = card.output.as_deref().unwrap_or("(no output)");
                let output_preview = if output.len() > 500 {
                    format!("{}...\n({} bytes total)", &output[..500], output.len())
                } else {
                    output.to_string()
                };
                app.chat.push_system(format!(
                    "── Last Tool Call ──\n\
                     Tool: {}\n\
                     Input: {}\n\
                     Status: {status}\n\
                     Output:\n{output_preview}",
                    card.tool_name, card.input_preview
                ));
            } else {
                app.chat.push_system("No tool calls in this session.".to_string());
            }
        }

        SlashCommand::Mcp { action, args } => {
            let msg = match action.as_deref() {
                None | Some("list") => list_mcp_servers(),
                Some("add") => {
                    if let Some(a) = args {
                        mcp_add_server(&a)
                    } else {
                        "Usage: /mcp add <name> <command> [args...]".to_string()
                    }
                }
                Some("remove") => {
                    if let Some(name) = args {
                        mcp_remove_server(&name)
                    } else {
                        "Usage: /mcp remove <server-name>".to_string()
                    }
                }
                Some(other) => format!("Unknown /mcp action: {other}"),
            };
            app.chat.push_system(msg);
        }
        SlashCommand::Dev { action, target } => {
            let msg = match action.as_deref() {
                None => "Usage: /dev <install|start|open URL|screenshot|click SEL|type SEL TEXT|eval JS|test DESC|stop|status>".to_string(),
                Some("install") => {
                    app.chat.push_system("Installing Playwright...".to_string());
                    capture_command_output("npm", &["install", "-g", "playwright@latest"])
                }
                Some("status") => capture_command_output("npx", &["playwright", "--version"]),
                Some("start") => "Use /dev start in the REPL (browser control requires interactive mode).".to_string(),
                Some("stop") => "Use /dev stop in the REPL.".to_string(),
                Some("open") => {
                    if let Some(url) = target {
                        format!("Navigate to: {url} (use REPL for live browser control)")
                    } else {
                        "Usage: /dev open <url>".to_string()
                    }
                }
                Some("test") => {
                    let description = target.unwrap_or_else(|| "the current page".to_string());
                    let prompt = format!(
                        "Write a Playwright test for: {description}. Use `const {{ test, expect }} = require('@playwright/test');` format. Return only the test code."
                    );
                    app.submit_prompt_internal(prompt);
                    return true;
                }
                Some(other) => format!("/dev {other}: use REPL mode for live browser control"),
            };
            app.chat.push_system(msg);
        }

        // ── Knowledge & Exploration ──
        SlashCommand::Knowledge { query } => {
            if let Some(q) = query {
                app.chat.push_system(format!("[>] Searching knowledge base: {q}..."));
                app.status_bar.phase = tui_widgets::status_bar::AgentPhase::Thinking;

                let api_key = std::env::var("OPENANALYST_API_KEY")
                    .or_else(|_| std::env::var("OA_API_KEY"))
                    .unwrap_or_default();

                if api_key.is_empty() {
                    app.chat.push_system(
                        "[!] OPENANALYST_API_KEY not set.\n\
                         Set your key to access the knowledge base:\n\
                           export OPENANALYST_API_KEY=oa_...\n\
                         Falling back to AI-only answer...".to_string()
                    );
                    let prompt = format!(
                        "Answer this query as an expert consultant. Be specific, actionable, \
                         and practical with concrete steps:\n\n{q}"
                    );
                    app.submit_prompt_internal(prompt);
                    return true;
                }

                let kb_endpoint = std::env::var("OPENANALYST_KB_URL")
                    .unwrap_or_else(|_| "http://44.200.9.142:8420/v1/knowledge/query".to_string());

                let query_clone = q.clone();
                let tx = app.action_tx.clone();
                tokio::spawn(async move {
                    let result = kb_fetch(&kb_endpoint, &api_key, &query_clone).await;
                    let prompt = match result {
                        Ok(body) => format!(
                            "The user asked: \"{query_clone}\"\n\n\
                             The knowledge base returned these results:\n\
                             ```json\n{body}\n```\n\n\
                             Synthesize a comprehensive, actionable answer from these results. \
                             Include source citations [1], [2] etc. Be specific and practical."
                        ),
                        Err(_) => format!(
                            "Answer this query as an expert consultant. Be specific, actionable, \
                             and practical with concrete steps:\n\n{query_clone}"
                        ),
                    };
                    let _ = tx.send(events::Action::SubmitPrompt { text: prompt, effort_budget: None, model_override: None }).await;
                });
            } else {
                app.chat.push_system(
                    "OpenAnalyst Knowledge Base\n\n\
                     Usage: /knowledge <query>\n\
                     Example: /knowledge how to create Meta Ads strategy for D2C\n\n\
                     Searches the hosted knowledge base for expert strategies,\n\
                     course insights, and actionable guidance.\n\n\
                     Requires OPENANALYST_API_KEY environment variable.".to_string()
                );
            }
        }
        SlashCommand::Explore { target } => {
            if let Some(url) = target {
                app.chat.push_system(format!("[>] Exploring repository: {url}..."));

                let target_clone = url.clone();
                let is_local = url == "." || std::path::Path::new(&url).is_dir();

                if is_local {
                    // Local repo — gather git data and send to LLM
                    let path = target_clone.clone();
                    let log = capture_command_output("git", &["-C", &path, "log", "--oneline", "--no-merges", "-50"]);
                    let stats = capture_command_output("git", &["-C", &path, "log", "--oneline", "--stat", "--no-merges", "-20"]);
                    let authors = capture_command_output("git", &["-C", &path, "shortlog", "-sn", "--no-merges", "-10"]);
                    let prompt = format!(
                        "You are analyzing a local repository at `{path}`. Based on the data below, provide:\n\
                         1. **Architecture Overview** — key modules, structure, entry points\n\
                         2. **Tech Stack** — languages, frameworks, build tools\n\
                         3. **Development Patterns** — active areas, commit themes\n\
                         4. **Key Features** — what this project does\n\
                         5. **Health Assessment** — commit frequency, contributor diversity\n\n\
                         Be concise and specific.\n\n\
                         --- RECENT COMMITS ---\n```\n{log}\n```\n\n\
                         --- FILE CHANGES ---\n```\n{stats}\n```\n\n\
                         --- CONTRIBUTORS ---\n```\n{authors}\n```"
                    );
                    app.submit_prompt_internal(prompt);
                } else {
                    // GitHub repo — use gh API in spawned task
                    let repo_slug = if url.contains("github.com") {
                        url.trim_end_matches('/')
                            .trim_end_matches(".git")
                            .rsplit("github.com/")
                            .next()
                            .unwrap_or(&url)
                            .to_string()
                    } else {
                        url.clone()
                    };

                    let tx = app.action_tx.clone();
                    tokio::spawn(async move {
                        let commits = tokio::task::spawn_blocking({
                            let slug = repo_slug.clone();
                            move || capture_command_output("gh", &[
                                "api", &format!("repos/{slug}/commits?per_page=50"),
                                "--jq", ".[] | \"\\(.sha[0:7]) \\(.commit.message | split(\"\\n\") | .[0])\""
                            ])
                        }).await.unwrap_or_default();

                        let languages = tokio::task::spawn_blocking({
                            let slug = repo_slug.clone();
                            move || capture_command_output("gh", &[
                                "api", &format!("repos/{slug}/languages")
                            ])
                        }).await.unwrap_or_default();

                        let tree = tokio::task::spawn_blocking({
                            let slug = repo_slug.clone();
                            move || capture_command_output("gh", &[
                                "api", &format!("repos/{slug}/git/trees/HEAD"),
                                "--jq", ".tree[] | \"\\(.type) \\(.path)\""
                            ])
                        }).await.unwrap_or_default();

                        let prompt = format!(
                            "You are analyzing the GitHub repository **{repo_slug}**. Provide:\n\
                             1. **What This Project Does** — purpose, key features\n\
                             2. **Architecture** — module structure, key directories\n\
                             3. **Tech Stack** — languages, frameworks\n\
                             4. **Development Activity** — active areas, commit patterns\n\
                             5. **Community** — health assessment\n\n\
                             Be concise and specific.\n\n\
                             --- LANGUAGES ---\n{languages}\n\n\
                             --- ROOT TREE ---\n{tree}\n\n\
                             --- RECENT COMMITS ---\n```\n{commits}\n```"
                        );
                        let _ = tx.send(events::Action::SubmitPrompt { text: prompt, effort_budget: None, model_override: None }).await;
                    });
                }
            } else {
                app.chat.push_system(
                    "Smart Repo Explorer\n\n\
                     Usage: /explore <github-url-or-local-path>\n\
                     Examples:\n\
                       /explore https://github.com/rust-lang/rust\n\
                       /explore owner/repo\n\
                       /explore .                     (current directory)\n\n\
                     Analyzes a repository from its git history to produce:\n\
                       - Architecture overview & tech stack\n\
                       - Commit patterns & active areas\n\
                       - Key contributors & development velocity".to_string()
                );
            }
        }

        // ── Claude Code parity ──
        SlashCommand::Doctor => {
            let output = run_doctor();
            app.chat.push_system(output);
        }
        SlashCommand::Login => {
            let output = show_login_status();
            app.chat.push_system(output);
        }
        SlashCommand::Logout => {
            // Clear credentials.json
            let config_dir = std::env::var("OPENANALYST_CONFIG_HOME")
                .or_else(|_| std::env::var("HOME").map(|h| format!("{h}/.openanalyst")))
                .or_else(|_| std::env::var("USERPROFILE").map(|h| format!("{h}/.openanalyst")))
                .unwrap_or_else(|_| ".openanalyst".to_string());
            let creds_path = std::path::Path::new(&config_dir).join("credentials.json");
            if creds_path.exists() {
                let _ = std::fs::remove_file(&creds_path);
                app.chat.push_system(format!("Credentials cleared: {}\nRun `openanalyst login` from the terminal to re-authenticate.", creds_path.display()));
            } else {
                app.chat.push_system("No saved credentials to clear.".to_string());
            }
        }
        SlashCommand::Vim => {
            app.chat.push_system("Vim mode: toggle with Ctrl+V in the input editor.".to_string());
        }
        SlashCommand::Think { prompt } => {
            let text = prompt.unwrap_or_else(|| "the next question".to_string());
            let p = format!("Think deeply and step-by-step about this before answering:\n\n{text}");
            app.submit_prompt_internal(p);
        }
        SlashCommand::Effort { category, level } => {
            use orchestrator::router::{ActionCategory, EffortLevel};
            match (category, level) {
                // /effort <category> <level> — set effort for one category
                (Some(cat_str), Some(lvl_str)) => {
                    if let (Some(cat), Some(lvl)) = (ActionCategory::from_str_opt(&cat_str), EffortLevel::from_str_opt(&lvl_str)) {
                        app.router.table.set_effort(cat, lvl);
                        app.chat.push_system(format!(
                            "Effort for {cat} set to: {lvl} ({} tokens)",
                            lvl.thinking_budget()
                        ));
                    } else {
                        app.chat.push_system(format!(
                            "Unknown category or level.\n\
                             Categories: explore, research, code, write\n\
                             Levels: low (1K), medium (8K), high (32K), max (128K)"
                        ));
                    }
                }
                // /effort <level> — set effort globally
                (None, Some(lvl_str)) => {
                    if let Some(lvl) = EffortLevel::from_str_opt(&lvl_str) {
                        app.router.table.set_effort_all(lvl);
                        app.chat.push_system(format!(
                            "Effort set globally to: {lvl} ({} tokens)\nAll categories updated.",
                            lvl.thinking_budget()
                        ));
                    } else {
                        app.chat.push_system(format!(
                            "Unknown effort level: {lvl_str}\n\
                             Options: low (1K), medium (8K), high (32K), max (128K)\n\
                             Per-category: /effort <category> <level>"
                        ));
                    }
                }
                // /effort <category> — show that category's current config
                (Some(cat_str), None) => {
                    if let Some(cat) = ActionCategory::from_str_opt(&cat_str) {
                        let profile = app.router.table.get(cat);
                        let model = app.router.resolver.resolve(profile.model_tier);
                        app.chat.push_system(format!(
                            "{cat}: effort={}, tier={}, model={model}",
                            profile.effort, profile.model_tier
                        ));
                    } else {
                        app.chat.push_system(format!(
                            "Unknown category: {cat_str}\n\
                             Options: explore, research, code, write"
                        ));
                    }
                }
                // /effort — show routing table
                (None, None) => {
                    let table = app.router.render_table();
                    app.chat.push_system(format!("Routing table:\n{table}\n\n\
                        Usage: /effort <level> (global) or /effort <category> <level>"));
                }
            }
        }
        SlashCommand::Route { args } => {
            use orchestrator::router::{ActionCategory, ModelTier};
            match args.as_deref() {
                // /route — show the routing table
                None | Some("") => {
                    let table = app.router.render_table();
                    app.chat.push_system(format!(
                        "Per-action routing table:\n{table}\n\n\
                         Edit: /route <category> <tier>\n\
                         Tiers: fast, balanced, capable\n\
                         Reset: /route reset"
                    ));
                }
                Some("reset") => {
                    app.router.table = orchestrator::router::RoutingTable::default();
                    let table = app.router.render_table();
                    app.chat.push_system(format!("Routing table reset to defaults:\n{table}"));
                }
                Some(rest) => {
                    let mut parts = rest.split_whitespace();
                    let cat_str = parts.next().unwrap_or("");
                    let tier_str = parts.next();
                    if let Some(cat) = ActionCategory::from_str_opt(cat_str) {
                        if let Some(tier_s) = tier_str {
                            if let Some(tier) = ModelTier::from_str_opt(tier_s) {
                                app.router.table.set_tier(cat, tier);
                                let model = app.router.resolver.resolve(tier);
                                app.chat.push_system(format!(
                                    "{cat} tier set to: {tier} ({model})"
                                ));
                            } else {
                                app.chat.push_system(format!(
                                    "Unknown tier: {tier_s}\nOptions: fast, balanced, capable"
                                ));
                            }
                        } else {
                            // Show single category
                            let profile = app.router.table.get(cat);
                            let model = app.router.resolver.resolve(profile.model_tier);
                            app.chat.push_system(format!(
                                "{cat}: tier={}, effort={}, model={model}",
                                profile.model_tier, profile.effort
                            ));
                        }
                    } else {
                        app.chat.push_system(format!(
                            "Unknown category: {cat_str}\n\
                             Options: explore, research, code, write"
                        ));
                    }
                }
            }
        }
        SlashCommand::Context => {
            let tokens = app.status_bar.total_tokens;
            app.chat.push_system(format!("Context: ~{tokens} tokens used"));
        }
        SlashCommand::Changelog { since } => {
            let tag = since.unwrap_or_else(|| "HEAD~20".to_string());
            let log = capture_command_output("git", &["log", "--oneline", "--no-merges", "-20"]);
            let prompt = format!("Generate a changelog from these commits since {tag}:\n```\n{log}\n```");
            app.submit_prompt_internal(prompt);
        }
        SlashCommand::AddDir { path } => {
            if let Some(dir) = path {
                app.chat.push_system(format!("Adding directory to context: {dir}"));
                let prompt = format!("I'm adding the directory `{dir}` to our conversation context. List its key files.");
                app.submit_prompt_internal(prompt);
            } else {
                app.chat.push_system("Usage: /add-dir <directory-path>".to_string());
            }
        }

        // ── TUI control commands ──
        SlashCommand::Exit => {
            app.chat.push_system("Saving session and exiting...".to_string());
            app.should_quit = true;
        }
        SlashCommand::Sidebar => {
            app.toggle_sidebar();
            let state = if app.sidebar_visible { "shown" } else { "hidden" };
            app.chat.push_system(format!("Sidebar {state}. (Ctrl+B to toggle)"));
        }
        SlashCommand::Swarm { task } => {
            if let Some(task_text) = task {
                app.chat.push_system(format!("Spawning agent swarm for: {task_text}"));

                // Phase 1: Spawn an Explore agent to gather context
                let explore_prompt = format!(
                    "You are a fast exploration agent. Quickly scan the codebase to understand the structure \
                     relevant to this task. List key files, functions, and patterns. Be concise.\n\nTask: {task_text}"
                );

                // Phase 2: Spawn a Plan agent to design the approach
                let plan_prompt = format!(
                    "You are a planning agent. Design a step-by-step implementation plan for this task. \
                     Identify files to modify, functions to change, and potential risks. Be thorough.\n\nTask: {task_text}"
                );

                // Send both as sub-agent spawn requests via the orchestrator
                let tx = app.action_tx.clone();
                let spawn_tx = tx.clone();
                let task_for_primary = task_text.clone();

                // Spawn explore + plan in parallel, then primary executes
                tokio::spawn(async move {
                    // First: explore (fast model)
                    let _ = spawn_tx.send(events::Action::SubmitPrompt {
                        text: explore_prompt,
                        effort_budget: Some(1_024), // low effort for exploration
                        model_override: Some("claude-haiku-4-5".to_string()),
                    }).await;
                });

                // Queue the plan and then the actual task
                app.pending_queue.push(plan_prompt);
                app.pending_queue.push(format!(
                    "Now execute this task based on the exploration and planning above:\n\n{task_for_primary}"
                ));

                app.sidebar_state.update_agent(
                    "swarm-explore".to_string(),
                    events::AgentType::Explore,
                    format!("Scan: {}", &task_text[..task_text.len().min(30)]),
                    events::AgentStatus::Running,
                );
                app.sidebar_state.update_agent(
                    "swarm-plan".to_string(),
                    events::AgentType::Plan,
                    format!("Plan: {}", &task_text[..task_text.len().min(30)]),
                    events::AgentStatus::Pending,
                );
            } else {
                app.chat.push_system("Usage: /swarm <task description>".to_string());
            }
        }

        SlashCommand::OpenAnalyst { task, goal, criteria, schedule, max_turns } => {
            if let Some(task_text) = task {
                let config = orchestrator::autonomous::AutonomousConfig {
                    task: task_text.clone(),
                    goal: goal.clone(),
                    criteria: criteria.clone(),
                    schedule: schedule.clone(),
                    max_turns: max_turns.unwrap_or(30),
                    turns_used: 0,
                };

                // Show config in chat
                app.chat.push_system(format!(
                    "🤖 Autonomous agent started\n{}",
                    config.status_summary()
                ));

                // Track in sidebar
                app.sidebar_state.update_agent(
                    "oa-auto".to_string(),
                    events::AgentType::General,
                    format!("Auto: {}", &task_text[..task_text.len().min(30)]),
                    events::AgentStatus::Running,
                );

                // Build the autonomous prompt and send
                let prompt = config.build_autonomous_prompt();

                // If criteria provided, add verification reminder
                if let Some(ref crit) = criteria {
                    // After each turn completes, the user can check criteria via
                    // the criteria check. For now, inject it into the prompt.
                    let verify_prompt = format!(
                        "{prompt}\n\n\
                         IMPORTANT: After completing your changes, run `{crit}` to verify. \
                         If it fails, analyze the output and fix the issue. \
                         Repeat until the criteria passes or you've exhausted approaches."
                    );
                    app.submit_prompt_internal(verify_prompt);
                } else {
                    app.submit_prompt_internal(prompt);
                }

                if schedule.is_some() {
                    app.chat.push_system(
                        "Note: Schedule support requires the remote trigger system. \
                         Use `openanalyst schedule` CLI to set up recurring runs.".to_string()
                    );
                }
            } else {
                app.chat.push_system(
                    "Usage: /openanalyst <task> [--goal <text>] [--criteria <cmd>] [--max-turns N]\n\n\
                     Examples:\n\
                     /openanalyst fix all failing tests --criteria \"cargo test\"\n\
                     /oa refactor auth to async --goal \"all auth fns are async\" --criteria \"cargo build\"\n\
                     /openanalyst add caching layer --max-turns 20".to_string()
                );
            }
        }

        SlashCommand::Ask { question } => {
            if let Some(q) = question {
                // Quick question — instruct the model to answer directly, no tools
                let prompt = format!(
                    "Answer this question directly and concisely. Do NOT use any tools. \
                     Do NOT read files, run commands, or search. Just answer from your knowledge:\n\n{q}"
                );
                app.submit_prompt_internal(prompt);
            } else {
                app.chat.push_system("Usage: /ask <question>\nQuick question — no tools, fast response.".to_string());
            }
        }

        SlashCommand::UserPrompt { prompt } => {
            if let Some(p) = prompt {
                // Full user message with tools enabled — just send it directly
                app.submit_prompt_internal(p);
            } else {
                app.chat.push_system("Usage: /user-prompt <message>\nInject a user message with full tool access.".to_string());
            }
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

// ═══════════════════════════════════════════════════════════════════
//  Real implementations for previously-stubbed commands
// ═══════════════════════════════════════════════════════════════════

/// /init — create OPENANALYST.md in the current directory.
fn run_init() -> String {
    let path = std::path::Path::new("OPENANALYST.md");
    if path.exists() {
        return format!("OPENANALYST.md already exists at {}", std::env::current_dir().unwrap_or_default().display());
    }
    let template = "\
# Project Instructions\n\n\
This file provides context to the AI agent about this project.\n\n\
## Overview\n\n\
Describe your project here.\n\n\
## Key Files\n\n\
- `src/` — source code\n\n\
## Conventions\n\n\
- Add project-specific coding conventions here.\n\
";
    match std::fs::write(path, template) {
        Ok(()) => format!("Created OPENANALYST.md in {}", std::env::current_dir().unwrap_or_default().display()),
        Err(e) => format!("Failed to create OPENANALYST.md: {e}"),
    }
}

/// /config — display environment, model, and provider configuration.
fn build_config_output(section: Option<&str>) -> String {
    let mut out = String::new();
    let show_all = section.is_none();
    let sec = section.unwrap_or("");

    if show_all || sec == "env" {
        out.push_str("── Environment ──\n");
        for var in &[
            "OPENANALYST_API_KEY", "OPENANALYST_AUTH_TOKEN",
            "ANTHROPIC_API_KEY", "OPENAI_API_KEY", "GEMINI_API_KEY",
            "XAI_API_KEY", "OPENROUTER_API_KEY", "BEDROCK_API_KEY",
            "STABILITY_API_KEY",
        ] {
            let val = std::env::var(var).ok().filter(|v| !v.is_empty());
            let display = match &val {
                Some(v) if v.len() > 8 => format!("{}...{}", &v[..4], &v[v.len()-4..]),
                Some(_) => "****".to_string(),
                None => "(not set)".to_string(),
            };
            out.push_str(&format!("  {var} = {display}\n"));
        }
        out.push('\n');
    }

    if show_all || sec == "model" {
        out.push_str("── Model ──\n");
        let model = std::env::var("OPENANALYST_MODEL")
            .or_else(|_| std::env::var("OPENANALYST_DEFAULT_MODEL"))
            .unwrap_or_else(|_| "(default)".to_string());
        out.push_str(&format!("  Active model: {model}\n\n"));
    }

    if show_all || sec == "paths" {
        out.push_str("── Paths ──\n");
        let config_home = std::env::var("OPENANALYST_CONFIG_HOME")
            .or_else(|_| std::env::var("HOME").map(|h| format!("{h}/.openanalyst")))
            .or_else(|_| std::env::var("USERPROFILE").map(|h| format!("{h}\\.openanalyst")))
            .unwrap_or_else(|_| "~/.openanalyst".to_string());
        out.push_str(&format!("  Config home:   {config_home}\n"));
        out.push_str(&format!("  .env file:     {config_home}/.env\n"));
        out.push_str(&format!("  Credentials:   {config_home}/credentials.json\n"));
        out.push_str(&format!("  Working dir:   {}\n", std::env::current_dir().unwrap_or_default().display()));
        out.push('\n');
    }

    if out.is_empty() {
        format!("Unknown config section: {sec}\nAvailable: env, model, paths")
    } else {
        out
    }
}

/// /memory — find and display OPENANALYST.md files.
fn build_memory_output() -> String {
    let mut found = Vec::new();
    let cwd = std::env::current_dir().unwrap_or_default();

    // Search cwd and parents for OPENANALYST.md
    let mut dir = Some(cwd.as_path());
    while let Some(d) = dir {
        let candidate = d.join("OPENANALYST.md");
        if candidate.exists() {
            if let Ok(content) = std::fs::read_to_string(&candidate) {
                let preview: String = content.lines().take(20).collect::<Vec<_>>().join("\n");
                let lines = content.lines().count();
                found.push(format!(
                    "── {} ({} lines) ──\n{}{}\n",
                    candidate.display(),
                    lines,
                    preview,
                    if lines > 20 { "\n  ..." } else { "" }
                ));
            }
        }
        // Also check .openanalyst/OPENANALYST.md
        let alt = d.join(".openanalyst").join("OPENANALYST.md");
        if alt.exists() {
            if let Ok(content) = std::fs::read_to_string(&alt) {
                let preview: String = content.lines().take(10).collect::<Vec<_>>().join("\n");
                found.push(format!("── {} ──\n{}\n", alt.display(), preview));
            }
        }
        dir = d.parent();
        // Don't walk above 5 levels
        if found.len() >= 5 { break; }
    }

    if found.is_empty() {
        "No OPENANALYST.md files found.\nRun /init to create one in the current directory.".to_string()
    } else {
        format!("Loaded {} instruction file(s):\n\n{}", found.len(), found.join("\n"))
    }
}

/// /session list — list session files from .openanalyst/sessions/.
fn list_sessions() -> String {
    let sessions_dir = std::path::Path::new(".openanalyst").join("sessions");
    if !sessions_dir.exists() {
        return "No saved sessions found.".to_string();
    }
    let mut entries: Vec<_> = std::fs::read_dir(&sessions_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();
    if entries.is_empty() {
        return "No saved sessions found.".to_string();
    }
    // Sort by modified time (newest first)
    entries.sort_by(|a, b| {
        b.metadata().and_then(|m| m.modified()).ok()
            .cmp(&a.metadata().and_then(|m| m.modified()).ok())
    });
    let mut out = format!("Sessions ({}):\n\n", entries.len());
    for (i, entry) in entries.iter().take(20).enumerate() {
        let name = entry.file_name();
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        out.push_str(&format!("  {}. {} ({:.1} KB)\n", i + 1, name.to_string_lossy(), size as f64 / 1024.0));
    }
    if entries.len() > 20 {
        out.push_str(&format!("  ...and {} more\n", entries.len() - 20));
    }
    out.push_str("\nUse /session switch <filename> or /resume <path> to load.");
    out
}

/// /resume and /session switch — load a session JSON into the chat.
fn load_session_into_chat(app: &mut crate::app::App, path: &str) -> Result<usize, String> {
    // Try the path directly, then in .openanalyst/sessions/
    let file_path = if std::path::Path::new(path).exists() {
        std::path::PathBuf::from(path)
    } else {
        let sessions_path = std::path::Path::new(".openanalyst").join("sessions").join(path);
        if sessions_path.exists() {
            sessions_path
        } else {
            return Err(format!("Session file not found: {path}"));
        }
    };

    let content = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
    let session: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    // Clear current chat
    app.chat.messages.clear();
    app.chat.scroll_offset = 0;
    app.chat.focused_message = None;

    // Session migration: detect version and normalize
    let version = session.get("version").and_then(|v| v.as_u64()).unwrap_or(1);
    let messages = migrate_session_messages(&session, version);

    let count = messages.len();
    for msg in &messages {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("system");
        let text = extract_message_text(msg);
        if text.is_empty() { continue; }
        match role {
            "user" => app.chat.push_user(text),
            "assistant" => {
                app.chat.start_assistant();
                app.chat.push_delta(&text);
                app.chat.finish_assistant();
            }
            "tool_call" => {
                let tool = msg.get("tool_name").and_then(|t| t.as_str()).unwrap_or("tool");
                let input = msg.get("input").and_then(|i| i.as_str()).unwrap_or("");
                app.chat.push_system(format!("[{tool}] {input}"));
            }
            _ => app.chat.push_system(text),
        }
    }

    // Restore token count if available
    if let Some(tokens) = session.get("tokens").and_then(|t| t.as_u64()) {
        app.status_bar.total_tokens = tokens;
    }

    Ok(count)
}

/// Migrate session messages across versions.
/// v1: flat array of {role, content} — content may be string or object
/// v2: same but with tool_call role and structured fields
/// Future: any new fields get defaults
fn migrate_session_messages(session: &serde_json::Value, version: u64) -> Vec<serde_json::Value> {
    let messages = session.get("messages")
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();

    if version >= 2 {
        return messages;
    }

    // v1 migration: normalize content formats
    messages.into_iter().map(|mut msg| {
        // v1 might have "content" as an array of content blocks (Anthropic format)
        if let Some(content) = msg.get("content") {
            if content.is_array() {
                // Flatten content blocks to text
                let text = content.as_array().unwrap().iter()
                    .filter_map(|b| {
                        b.get("text").and_then(|t| t.as_str()).map(ToOwned::to_owned)
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                msg.as_object_mut().unwrap().insert("content".to_string(), serde_json::Value::String(text));
            }
        }
        msg
    }).collect()
}

/// Extract text from a message, handling both string and array content formats.
fn extract_message_text(msg: &serde_json::Value) -> String {
    msg.get("content")
        .and_then(|c| {
            c.as_str().map(ToOwned::to_owned).or_else(|| {
                c.as_array().map(|blocks| {
                    blocks.iter()
                        .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                        .collect::<Vec<_>>()
                        .join("\n")
                })
            })
        })
        .unwrap_or_default()
}

/// /export — write chat messages to a markdown file.
fn export_session(messages: &[crate::panels::chat::ChatMessage], dest: &str) -> String {
    use crate::panels::chat::ChatMessage;
    let mut md = String::from("# OpenAnalyst CLI — Session Export\n\n");
    let mut count = 0u32;
    for msg in messages {
        match msg {
            ChatMessage::User { text } => {
                md.push_str(&format!("### User\n\n{text}\n\n---\n\n"));
                count += 1;
            }
            ChatMessage::Assistant { markdown, .. } => {
                md.push_str(&format!("### Assistant\n\n{}\n\n---\n\n", markdown.raw()));
                count += 1;
            }
            ChatMessage::System { text } => {
                md.push_str(&format!("### System\n\n{text}\n\n---\n\n"));
                count += 1;
            }
            ChatMessage::ToolCall { card } => {
                md.push_str(&format!("### Tool Call: {}\n\n```\n{}\n```\n\n---\n\n", card.tool_name, card.input_preview));
                count += 1;
            }
            ChatMessage::FileOutput { path, description, .. } => {
                md.push_str(&format!("### File Output\n\n{description}\nPath: {path}\n\n---\n\n"));
                count += 1;
            }
        }
    }
    match std::fs::write(dest, &md) {
        Ok(()) => format!("Session exported to {dest} ({count} messages, {:.1} KB)", md.len() as f64 / 1024.0),
        Err(e) => format!("Failed to export: {e}"),
    }
}

/// /doctor — run provider connectivity diagnostics.
fn run_doctor() -> String {
    let mut out = String::from("── OpenAnalyst CLI Diagnostics ──\n\n");

    // Check binary
    out.push_str("Binary:      openanalyst v1.0.89\n");
    out.push_str(&format!("Working dir: {}\n", std::env::current_dir().unwrap_or_default().display()));
    out.push_str(&format!("OS:          {}\n\n", std::env::consts::OS));

    // Check provider keys
    out.push_str("── Provider Keys ──\n\n");
    let providers: &[(&str, &str)] = &[
        ("OPENANALYST_API_KEY", "OpenAnalyst"),
        ("OPENANALYST_AUTH_TOKEN", "OpenAnalyst (OAuth)"),
        ("ANTHROPIC_API_KEY", "Anthropic / Claude"),
        ("OPENAI_API_KEY", "OpenAI / Codex"),
        ("GEMINI_API_KEY", "Google Gemini"),
        ("XAI_API_KEY", "xAI / Grok"),
        ("OPENROUTER_API_KEY", "OpenRouter"),
        ("BEDROCK_API_KEY", "Amazon Bedrock"),
        ("STABILITY_API_KEY", "Stability AI"),
    ];
    let mut configured = 0u32;
    for (var, name) in providers {
        let set = std::env::var(var).ok().filter(|v| !v.is_empty()).is_some();
        let icon = if set { "\u{2713}" } else { "\u{2717}" };
        out.push_str(&format!("  {icon} {name:<22} {var}\n"));
        if set { configured += 1; }
    }
    out.push_str(&format!("\n  {configured} provider(s) configured.\n\n"));

    // Check config files
    out.push_str("── Config Files ──\n\n");
    let config_home = std::env::var("OPENANALYST_CONFIG_HOME")
        .or_else(|_| std::env::var("HOME").map(|h| format!("{h}/.openanalyst")))
        .or_else(|_| std::env::var("USERPROFILE").map(|h| format!("{h}\\.openanalyst")))
        .unwrap_or_else(|_| "~/.openanalyst".to_string());
    for file in &[".env", "credentials.json", "settings.json"] {
        let p = std::path::Path::new(&config_home).join(file);
        let icon = if p.exists() { "\u{2713}" } else { "\u{2717}" };
        out.push_str(&format!("  {icon} {}\n", p.display()));
    }
    let oa_md = std::path::Path::new("OPENANALYST.md");
    let icon = if oa_md.exists() { "\u{2713}" } else { "\u{2717}" };
    out.push_str(&format!("  {icon} OPENANALYST.md (project)\n"));

    // Check git
    out.push_str("\n── Git ──\n\n");
    let git_ok = std::process::Command::new("git").args(["rev-parse", "--git-dir"]).output()
        .is_ok_and(|o| o.status.success());
    out.push_str(&format!("  {} Git repository\n", if git_ok { "\u{2713}" } else { "\u{2717}" }));

    out
}

/// /mcp list — display configured MCP servers from settings.
fn list_mcp_servers() -> String {
    // Try to read from .openanalyst/settings.json and ~/.openanalyst/settings.json
    let mut servers = Vec::new();

    for base in &[".openanalyst/settings.json", ".openanalyst/settings.local.json"] {
        if let Ok(content) = std::fs::read_to_string(base) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(mcp) = val.get("mcpServers").and_then(|v| v.as_object()) {
                    for (name, config) in mcp {
                        let cmd = config.get("command").and_then(|v| v.as_str()).unwrap_or("(unknown)");
                        servers.push(format!("  \u{25A0} {name}  →  {cmd}"));
                    }
                }
            }
        }
    }

    // Also check ~/.openanalyst/settings.json
    let home_settings = std::env::var("OPENANALYST_CONFIG_HOME")
        .or_else(|_| std::env::var("HOME").map(|h| format!("{h}/.openanalyst")))
        .or_else(|_| std::env::var("USERPROFILE").map(|h| format!("{h}\\.openanalyst")))
        .unwrap_or_default();
    let home_path = std::path::Path::new(&home_settings).join("settings.json");
    if let Ok(content) = std::fs::read_to_string(&home_path) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(mcp) = val.get("mcpServers").and_then(|v| v.as_object()) {
                for (name, config) in mcp {
                    let cmd = config.get("command").and_then(|v| v.as_str()).unwrap_or("(unknown)");
                    let entry = format!("  \u{25A0} {name}  →  {cmd}");
                    if !servers.contains(&entry) {
                        servers.push(entry);
                    }
                }
            }
        }
    }

    if servers.is_empty() {
        "No MCP servers configured.\nAdd servers in .openanalyst/settings.json under \"mcpServers\".".to_string()
    } else {
        format!("MCP Servers ({}):\n\n{}", servers.len(), servers.join("\n"))
    }
}

/// /login — show provider auth status and guide user.
fn show_login_status() -> String {
    let mut out = String::from("── Provider Auth Status ──\n\n");
    let providers: &[(&str, &str)] = &[
        ("OPENANALYST_AUTH_TOKEN", "OpenAnalyst"),
        ("ANTHROPIC_API_KEY", "Anthropic / Claude"),
        ("OPENAI_API_KEY", "OpenAI / Codex"),
        ("GEMINI_API_KEY", "Google Gemini"),
        ("XAI_API_KEY", "xAI / Grok"),
        ("OPENROUTER_API_KEY", "OpenRouter"),
        ("BEDROCK_API_KEY", "Amazon Bedrock"),
    ];
    for (var, name) in providers {
        let set = std::env::var(var).ok().filter(|v| !v.is_empty()).is_some();
        let icon = if set { "\u{2713}" } else { "\u{2717}" };
        out.push_str(&format!("  {icon} {name}\n"));
    }
    out.push_str("\nTo add or switch providers:\n");
    out.push_str("  • Run `openanalyst login` from the terminal\n");
    out.push_str("  • Or edit ~/.openanalyst/.env\n");
    out
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

/// Fetch knowledge base results from the hosted API.
async fn kb_fetch(endpoint: &str, api_key: &str, query: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let payload = serde_json::json!({
        "query": query,
        "mode": "progressive",
        "max_results": 10,
        "synthesize": false
    });

    let resp = client
        .post(endpoint)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    let status = resp.status();
    let body = resp.text().await.map_err(|e| format!("Read body failed: {e}"))?;

    if status.is_success() {
        Ok(body)
    } else {
        Err(format!("HTTP {status}: {body}"))
    }
}

/// Compact chat messages — remove tool call details, merge consecutive system messages,
/// and trim old messages to keep the session manageable.
fn compact_chat_messages(app: &mut App) {
    let messages = &mut app.chat.messages;

    // Phase 1: collapse tool calls into one-line summaries
    for msg in messages.iter_mut() {
        if let ChatMessage::ToolCall { card } = msg {
            // Replace with a system summary
            let status_str = match &card.status {
                tui_widgets::ToolCallStatus::Completed { duration } => format!("ok {:.1}s", duration.as_secs_f64()),
                tui_widgets::ToolCallStatus::Failed { duration } => format!("err {:.1}s", duration.as_secs_f64()),
                tui_widgets::ToolCallStatus::Running { .. } => "running".to_string(),
            };
            let summary = format!("[{}] {} → {status_str}", card.tool_name, card.input_preview);
            *msg = ChatMessage::System { text: summary };
        }
    }

    // Phase 2: merge consecutive system messages
    let mut merged = Vec::with_capacity(messages.len());
    for msg in messages.drain(..) {
        if let ChatMessage::System { text } = &msg {
            if let Some(ChatMessage::System { text: prev }) = merged.last_mut() {
                prev.push('\n');
                prev.push_str(text);
                continue;
            }
        }
        merged.push(msg);
    }

    // Phase 3: if still too many messages, keep the last N
    const MAX_AFTER_COMPACT: usize = 100;
    if merged.len() > MAX_AFTER_COMPACT {
        let removed = merged.len() - MAX_AFTER_COMPACT;
        merged.drain(..removed);
        merged.insert(
            0,
            ChatMessage::System {
                text: format!("({removed} older messages compacted)"),
            },
        );
    }

    *messages = merged;
}

/// Handle /plugins commands using the PluginManager.
fn handle_plugins_command(action: Option<&str>, target: Option<&str>) -> String {
    let config_home = resolve_config_home();
    let config = plugins::PluginManagerConfig::new(&config_home);
    let mut manager = plugins::PluginManager::new(config);

    match action {
        Some("list") | None => {
            match manager.list_plugins() {
                Ok(plugins) if plugins.is_empty() => {
                    "No plugins installed.\nUse /plugins install <path> to install.".to_string()
                }
                Ok(plugins) => {
                    let mut lines = vec![format!("Plugins ({}):", plugins.len())];
                    for p in &plugins {
                        let status = if p.enabled { "\u{2713}" } else { "\u{2717}" };
                        lines.push(format!(
                            "  {status} {} v{} ({})",
                            p.metadata.id, p.metadata.version, p.metadata.kind
                        ));
                    }
                    lines.join("\n")
                }
                Err(e) => format!("Error listing plugins: {e}"),
            }
        }
        Some("install") => {
            let source = target.unwrap_or("");
            if source.is_empty() {
                return "Usage: /plugins install <path-to-plugin>".to_string();
            }
            match manager.install(source) {
                Ok(outcome) => format!(
                    "Plugin installed: {} v{}",
                    outcome.plugin_id, outcome.version
                ),
                Err(e) => format!("Install failed: {e}"),
            }
        }
        Some("enable") => {
            let id = target.unwrap_or("");
            if id.is_empty() {
                return "Usage: /plugins enable <plugin-id>".to_string();
            }
            match manager.enable(id) {
                Ok(()) => format!("Plugin enabled: {id}"),
                Err(e) => format!("Enable failed: {e}"),
            }
        }
        Some("disable") => {
            let id = target.unwrap_or("");
            if id.is_empty() {
                return "Usage: /plugins disable <plugin-id>".to_string();
            }
            match manager.disable(id) {
                Ok(()) => format!("Plugin disabled: {id}"),
                Err(e) => format!("Disable failed: {e}"),
            }
        }
        Some("uninstall") => {
            let id = target.unwrap_or("");
            if id.is_empty() {
                return "Usage: /plugins uninstall <plugin-id>".to_string();
            }
            match manager.uninstall(id) {
                Ok(()) => format!("Plugin uninstalled: {id}"),
                Err(e) => format!("Uninstall failed: {e}"),
            }
        }
        Some(other) => format!("Unknown plugin action: {other}\nOptions: list, install, enable, disable, uninstall"),
    }
}

/// Add an MCP server to .openanalyst/settings.json.
fn mcp_add_server(args: &str) -> String {
    let mut parts = args.split_whitespace();
    let name = match parts.next() {
        Some(n) => n,
        None => return "Usage: /mcp add <name> <command> [args...]".to_string(),
    };
    let command = match parts.next() {
        Some(c) => c,
        None => return "Usage: /mcp add <name> <command> [args...]".to_string(),
    };
    let extra_args: Vec<&str> = parts.collect();

    let settings_path = std::path::Path::new(".openanalyst").join("settings.json");
    let _ = std::fs::create_dir_all(".openanalyst");

    let mut root = if let Ok(content) = std::fs::read_to_string(&settings_path) {
        serde_json::from_str::<serde_json::Value>(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let server_config = if extra_args.is_empty() {
        serde_json::json!({ "command": command })
    } else {
        serde_json::json!({ "command": command, "args": extra_args })
    };

    // Scope mutable borrows so `root` can be serialized afterward
    {
        let Some(root_obj) = root.as_object_mut() else {
            return "Error: settings.json root is not a JSON object.".to_string();
        };

        let mcp_servers = root_obj
            .entry("mcpServers")
            .or_insert_with(|| serde_json::json!({}));

        let Some(mcp_obj) = mcp_servers.as_object_mut() else {
            return "Error: \"mcpServers\" in settings.json is not a JSON object.".to_string();
        };
        mcp_obj.insert(name.to_string(), server_config);
    }

    match std::fs::write(&settings_path, serde_json::to_string_pretty(&root).unwrap_or_default()) {
        Ok(()) => format!(
            "MCP server '{name}' added → {command}\nSaved to {}\nRestart CLI to activate.",
            settings_path.display()
        ),
        Err(e) => format!("Failed to write settings: {e}"),
    }
}

/// Remove an MCP server from .openanalyst/settings.json.
fn mcp_remove_server(name: &str) -> String {
    let settings_path = std::path::Path::new(".openanalyst").join("settings.json");

    let mut root = if let Ok(content) = std::fs::read_to_string(&settings_path) {
        serde_json::from_str::<serde_json::Value>(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        return format!("No settings file found at {}", settings_path.display());
    };

    let removed = root
        .as_object_mut()
        .and_then(|obj| obj.get_mut("mcpServers"))
        .and_then(|mcp| mcp.as_object_mut())
        .and_then(|servers| servers.remove(name));

    match removed {
        Some(_) => {
            match std::fs::write(&settings_path, serde_json::to_string_pretty(&root).unwrap_or_default()) {
                Ok(()) => format!("MCP server '{name}' removed.\nRestart CLI to apply."),
                Err(e) => format!("Failed to write settings: {e}"),
            }
        }
        None => format!("MCP server '{name}' not found in settings."),
    }
}

/// Resolve the OpenAnalyst config home directory.
fn resolve_config_home() -> std::path::PathBuf {
    std::env::var("OPENANALYST_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| std::path::PathBuf::from(h).join(".openanalyst")))
        .or_else(|_| std::env::var("USERPROFILE").map(|h| std::path::PathBuf::from(h).join(".openanalyst")))
        .unwrap_or_else(|_| std::path::PathBuf::from(".openanalyst"))
}
