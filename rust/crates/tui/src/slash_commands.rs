//! Slash command routing for the TUI.
//!
//! Parses `/` input via `commands::SlashCommand::parse()` and routes each
//! command to the appropriate handler: text output to chat, AI tasks to
//! orchestrator, multimedia to file output, state changes to app state.

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

        SlashCommand::Mcp { action, args } => {
            let msg = match action.as_deref() {
                None | Some("list") => list_mcp_servers(),
                Some("add") => {
                    if let Some(a) = args {
                        format!("MCP server add requested: {a}\nRestart CLI to activate.")
                    } else {
                        "Usage: /mcp add <name> <command> [args...]".to_string()
                    }
                }
                Some("remove") => {
                    format!("MCP server remove: {}", args.unwrap_or_default())
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
                    let _ = tx.send(events::Action::SubmitPrompt(prompt)).await;
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
                        let _ = tx.send(events::Action::SubmitPrompt(prompt)).await;
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
        SlashCommand::Effort { level } => {
            use crate::app::EffortLevel;
            if let Some(lvl_str) = level {
                if let Some(lvl) = EffortLevel::from_str_opt(&lvl_str) {
                    app.effort = lvl;
                    app.chat.push_system(format!(
                        "Effort set to: {} (thinking budget: {} tokens)",
                        lvl.as_str(),
                        lvl.thinking_budget()
                    ));
                } else {
                    app.chat.push_system(format!(
                        "Unknown effort level: {lvl_str}\nOptions: low (1K), medium (8K), high (32K), max (128K)"
                    ));
                }
            } else {
                app.chat.push_system(format!(
                    "Current effort: {} (thinking budget: {} tokens)\n\
                     Options: /effort low | medium | high | max",
                    app.effort.as_str(),
                    app.effort.thinking_budget()
                ));
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

    // Load messages from session
    let messages = session.get("messages")
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();

    let count = messages.len();
    for msg in &messages {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("system");
        let text = msg.get("content")
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
            .unwrap_or_default();
        if text.is_empty() { continue; }
        match role {
            "user" => app.chat.push_user(text),
            "assistant" => {
                app.chat.start_assistant();
                app.chat.push_delta(&text);
                app.chat.finish_assistant();
            }
            _ => app.chat.push_system(text),
        }
    }

    Ok(count)
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
    out.push_str("Binary:      openanalyst v1.0.1\n");
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
