use std::fs;
use std::path::{Path, PathBuf};

const STARTER_OPENANALYST_JSON: &str = concat!(
    "{\n",
    "  \"permissions\": {\n",
    "    \"defaultMode\": \"dontAsk\"\n",
    "  }\n",
    "}\n",
);
const PROJECT_DOTENV_TEMPLATE: &str = r#"# ═══════════════════════════════════════════════════════════════════
#  OpenAnalyst CLI — Project Environment Configuration
# ═══════════════════════════════════════════════════════════════════
#
#  Project-level API keys. These take priority over ~/.openanalyst/.env.
#  This file is gitignored — safe to store keys here.
#  You can also use `openanalyst login` for interactive setup.
#
#  Docs: https://openanalyst.com/docs
# ═══════════════════════════════════════════════════════════════════

# ── Provider API Keys ─────────────────────────────────────────────
# Uncomment and add your key for each provider you want to use.

# OpenAnalyst (default provider — free tier available)
# OPENANALYST_API_KEY=
# OPENANALYST_AUTH_TOKEN=

# Anthropic / Claude (opus, sonnet, haiku)
# ANTHROPIC_API_KEY=sk-ant-...

# OpenAI (gpt-4o, o3, codex-mini)
# OPENAI_API_KEY=sk-...

# Google Gemini (gemini-2.5-pro, flash)
# GEMINI_API_KEY=AIza...

# xAI / Grok (grok-3, grok-mini)
# XAI_API_KEY=xai-...

# OpenRouter (350+ models via one key)
# OPENROUTER_API_KEY=sk-or-...

# Amazon Bedrock
# BEDROCK_API_KEY=

# Stability AI (image generation via /image)
# STABILITY_API_KEY=sk-...

# ── Self-Hosted / Local Models ────────────────────────────────────
# Connect to locally hosted models (Ollama, vLLM, LM Studio, text-generation-webui, etc.)
# Any OpenAI-compatible endpoint works here.

# Ollama (default: http://localhost:11434/v1)
# OLLAMA_BASE_URL=http://localhost:11434/v1
# OLLAMA_API_KEY=ollama

# Local OpenAI-compatible server (vLLM, LM Studio, LocalAI, etc.)
# LOCAL_LLM_BASE_URL=http://localhost:8000/v1
# LOCAL_LLM_API_KEY=

# Custom model name for local inference
# LOCAL_LLM_MODEL=

# ── Base URL Overrides (optional) ─────────────────────────────────
# Use these to point to custom/proxy endpoints.

# OPENANALYST_BASE_URL=
# ANTHROPIC_BASE_URL=
# OPENAI_BASE_URL=
# GEMINI_BASE_URL=
"#;

const GITIGNORE_COMMENT: &str = "# OpenAnalyst CLI local artifacts";
const GITIGNORE_ENTRIES: [&str; 4] = [
    ".openanalyst/settings.local.json",
    ".openanalyst/sessions/",
    ".openanalyst/.env",
    "OPENANALYST.local.md",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InitStatus {
    Created,
    Updated,
    Skipped,
}

impl InitStatus {
    #[must_use]
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Skipped => "skipped (already exists)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InitArtifact {
    pub(crate) name: &'static str,
    pub(crate) status: InitStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InitReport {
    pub(crate) project_root: PathBuf,
    pub(crate) artifacts: Vec<InitArtifact>,
}

impl InitReport {
    #[must_use]
    pub(crate) fn render(&self) -> String {
        let mut lines = vec![
            "Init".to_string(),
            format!("  Project          {}", self.project_root.display()),
        ];
        for artifact in &self.artifacts {
            lines.push(format!(
                "  {:<18} {}",
                artifact.name,
                artifact.status.label()
            ));
        }
        lines.push("  Next step        Review and tailor the generated guidance".to_string());
        lines.join("\n")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
struct RepoDetection {
    rust_workspace: bool,
    rust_root: bool,
    python: bool,
    package_json: bool,
    typescript: bool,
    nextjs: bool,
    react: bool,
    vite: bool,
    nest: bool,
    src_dir: bool,
    tests_dir: bool,
    rust_dir: bool,
}

pub(crate) fn initialize_repo(cwd: &Path) -> Result<InitReport, Box<dyn std::error::Error>> {
    let mut artifacts = Vec::new();

    let oa_dir = cwd.join(".openanalyst");
    artifacts.push(InitArtifact {
        name: ".openanalyst/",
        status: ensure_dir(&oa_dir)?,
    });

    // Create subdirectories matching the full project configuration structure
    for (name, subdir) in [
        ("sessions/", "sessions"),
        ("skills/", "skills"),
        ("commands/", "commands"),
        ("rules/", "rules"),
        ("agents/", "agents"),
        ("hooks/", "hooks"),
        ("output-styles/", "output-styles"),
    ] {
        artifacts.push(InitArtifact {
            name,
            status: ensure_dir(&oa_dir.join(subdir))?,
        });
    }

    // Create .openanalyst/.env with API key placeholders
    let oa_env = oa_dir.join(".env");
    artifacts.push(InitArtifact {
        name: ".env",
        status: write_file_if_missing(&oa_env, PROJECT_DOTENV_TEMPLATE)?,
    });

    let oa_json = cwd.join(".openanalyst.json");
    artifacts.push(InitArtifact {
        name: ".openanalyst.json",
        status: write_file_if_missing(&oa_json, STARTER_OPENANALYST_JSON)?,
    });

    let gitignore = cwd.join(".gitignore");
    artifacts.push(InitArtifact {
        name: ".gitignore",
        status: ensure_gitignore_entries(&gitignore)?,
    });

    let oa_md = cwd.join("OPENANALYST.md");
    let content = render_init_openanalyst_md(cwd);
    artifacts.push(InitArtifact {
        name: "OPENANALYST.md",
        status: write_file_if_missing(&oa_md, &content)?,
    });

    Ok(InitReport {
        project_root: cwd.to_path_buf(),
        artifacts,
    })
}

fn ensure_dir(path: &Path) -> Result<InitStatus, std::io::Error> {
    if path.is_dir() {
        return Ok(InitStatus::Skipped);
    }
    fs::create_dir_all(path)?;
    Ok(InitStatus::Created)
}

fn write_file_if_missing(path: &Path, content: &str) -> Result<InitStatus, std::io::Error> {
    if path.exists() {
        return Ok(InitStatus::Skipped);
    }
    fs::write(path, content)?;
    Ok(InitStatus::Created)
}

fn ensure_gitignore_entries(path: &Path) -> Result<InitStatus, std::io::Error> {
    if !path.exists() {
        let mut lines = vec![GITIGNORE_COMMENT.to_string()];
        lines.extend(GITIGNORE_ENTRIES.iter().map(|entry| (*entry).to_string()));
        fs::write(path, format!("{}\n", lines.join("\n")))?;
        return Ok(InitStatus::Created);
    }

    let existing = fs::read_to_string(path)?;
    let mut lines = existing.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
    let mut changed = false;

    if !lines.iter().any(|line| line == GITIGNORE_COMMENT) {
        lines.push(GITIGNORE_COMMENT.to_string());
        changed = true;
    }

    for entry in GITIGNORE_ENTRIES {
        if !lines.iter().any(|line| line == entry) {
            lines.push(entry.to_string());
            changed = true;
        }
    }

    if !changed {
        return Ok(InitStatus::Skipped);
    }

    fs::write(path, format!("{}\n", lines.join("\n")))?;
    Ok(InitStatus::Updated)
}

pub(crate) fn render_init_openanalyst_md(cwd: &Path) -> String {
    let detection = detect_repo(cwd);
    let mut lines = vec![
        "# OPENANALYST.md".to_string(),
        String::new(),
        "This file provides guidance to OpenAnalyst CLI (openanalyst.com) when working with code in this repository.".to_string(),
        String::new(),
    ];

    let detected_languages = detected_languages(&detection);
    let detected_frameworks = detected_frameworks(&detection);
    lines.push("## Detected stack".to_string());
    if detected_languages.is_empty() {
        lines.push("- No specific language markers were detected yet; document the primary language and verification commands once the project structure settles.".to_string());
    } else {
        lines.push(format!("- Languages: {}.", detected_languages.join(", ")));
    }
    if detected_frameworks.is_empty() {
        lines.push("- Frameworks: none detected from the supported starter markers.".to_string());
    } else {
        lines.push(format!(
            "- Frameworks/tooling markers: {}.",
            detected_frameworks.join(", ")
        ));
    }
    lines.push(String::new());

    let verification_lines = verification_lines(cwd, &detection);
    if !verification_lines.is_empty() {
        lines.push("## Verification".to_string());
        lines.extend(verification_lines);
        lines.push(String::new());
    }

    let structure_lines = repository_shape_lines(&detection);
    if !structure_lines.is_empty() {
        lines.push("## Repository shape".to_string());
        lines.extend(structure_lines);
        lines.push(String::new());
    }

    let framework_lines = framework_notes(&detection);
    if !framework_lines.is_empty() {
        lines.push("## Framework notes".to_string());
        lines.extend(framework_lines);
        lines.push(String::new());
    }

    lines.push("## Working agreement".to_string());
    lines.push("- Prefer small, reviewable changes and keep generated bootstrap files aligned with actual repo workflows.".to_string());
    lines.push("- Keep shared defaults in `.openanalyst.json`; reserve `.openanalyst/settings.local.json` for machine-local overrides.".to_string());
    lines.push("- Do not overwrite existing `OPENANALYST.md` content automatically; update it intentionally when repo workflows change.".to_string());
    lines.push(String::new());

    lines.join("\n")
}

fn detect_repo(cwd: &Path) -> RepoDetection {
    let package_json_contents = fs::read_to_string(cwd.join("package.json"))
        .unwrap_or_default()
        .to_ascii_lowercase();
    RepoDetection {
        rust_workspace: cwd.join("rust").join("Cargo.toml").is_file(),
        rust_root: cwd.join("Cargo.toml").is_file(),
        python: cwd.join("pyproject.toml").is_file()
            || cwd.join("requirements.txt").is_file()
            || cwd.join("setup.py").is_file(),
        package_json: cwd.join("package.json").is_file(),
        typescript: cwd.join("tsconfig.json").is_file()
            || package_json_contents.contains("typescript"),
        nextjs: package_json_contents.contains("\"next\""),
        react: package_json_contents.contains("\"react\""),
        vite: package_json_contents.contains("\"vite\""),
        nest: package_json_contents.contains("@nestjs"),
        src_dir: cwd.join("src").is_dir(),
        tests_dir: cwd.join("tests").is_dir(),
        rust_dir: cwd.join("rust").is_dir(),
    }
}

fn detected_languages(detection: &RepoDetection) -> Vec<&'static str> {
    let mut languages = Vec::new();
    if detection.rust_workspace || detection.rust_root {
        languages.push("Rust");
    }
    if detection.python {
        languages.push("Python");
    }
    if detection.typescript {
        languages.push("TypeScript");
    } else if detection.package_json {
        languages.push("JavaScript/Node.js");
    }
    languages
}

fn detected_frameworks(detection: &RepoDetection) -> Vec<&'static str> {
    let mut frameworks = Vec::new();
    if detection.nextjs {
        frameworks.push("Next.js");
    }
    if detection.react {
        frameworks.push("React");
    }
    if detection.vite {
        frameworks.push("Vite");
    }
    if detection.nest {
        frameworks.push("NestJS");
    }
    frameworks
}

fn verification_lines(cwd: &Path, detection: &RepoDetection) -> Vec<String> {
    let mut lines = Vec::new();
    if detection.rust_workspace {
        lines.push("- Run Rust verification from `rust/`: `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`".to_string());
    } else if detection.rust_root {
        lines.push("- Run Rust verification from the repo root: `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`".to_string());
    }
    if detection.python {
        if cwd.join("pyproject.toml").is_file() {
            lines.push("- Run the Python project checks declared in `pyproject.toml` (for example: `pytest`, `ruff check`, and `mypy` when configured).".to_string());
        } else {
            lines.push(
                "- Run the repo's Python test/lint commands before shipping changes.".to_string(),
            );
        }
    }
    if detection.package_json {
        lines.push("- Run the JavaScript/TypeScript checks from `package.json` before shipping changes (`npm test`, `npm run lint`, `npm run build`, or the repo equivalent).".to_string());
    }
    if detection.tests_dir && detection.src_dir {
        lines.push("- `src/` and `tests/` are both present; update both surfaces together when behavior changes.".to_string());
    }
    lines
}

fn repository_shape_lines(detection: &RepoDetection) -> Vec<String> {
    let mut lines = Vec::new();
    if detection.rust_dir {
        lines.push(
            "- `rust/` contains the Rust workspace and active CLI/runtime implementation."
                .to_string(),
        );
    }
    if detection.src_dir {
        lines.push("- `src/` contains source files that should stay consistent with generated guidance and tests.".to_string());
    }
    if detection.tests_dir {
        lines.push("- `tests/` contains validation surfaces that should be reviewed alongside code changes.".to_string());
    }
    lines
}

fn framework_notes(detection: &RepoDetection) -> Vec<String> {
    let mut lines = Vec::new();
    if detection.nextjs {
        lines.push("- Next.js detected: preserve routing/data-fetching conventions and verify production builds after changing app structure.".to_string());
    }
    if detection.react && !detection.nextjs {
        lines.push("- React detected: keep component behavior covered with focused tests and avoid unnecessary prop/API churn.".to_string());
    }
    if detection.vite {
        lines.push("- Vite detected: validate the production bundle after changing build-sensitive configuration or imports.".to_string());
    }
    if detection.nest {
        lines.push("- NestJS detected: keep module/provider boundaries explicit and verify controller/service wiring after refactors.".to_string());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::{initialize_repo, render_init_openanalyst_md};
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("openanalyst-init-{nanos}"))
    }

    #[test]
    fn initialize_repo_creates_expected_files_and_gitignore_entries() {
        let root = temp_dir();
        fs::create_dir_all(root.join("rust")).expect("create rust dir");
        fs::write(root.join("rust").join("Cargo.toml"), "[workspace]\n").expect("write cargo");

        let report = initialize_repo(&root).expect("init should succeed");
        let rendered = report.render();
        assert!(rendered.contains(".openanalyst/      created"));
        for subdir in ["sessions/", "skills/", "commands/", "rules/", "agents/", "hooks/", "output-styles/"] {
            assert!(rendered.contains(&format!("{:<18} created", subdir)), "missing: {subdir}");
            let dir_name = subdir.trim_end_matches('/');
            assert!(root.join(".openanalyst").join(dir_name).is_dir(), "dir missing: {dir_name}");
        }
        assert!(rendered.contains(".openanalyst.json  created"));
        assert!(rendered.contains(".gitignore         created"));
        assert!(rendered.contains("OPENANALYST.md     created"));
        assert!(root.join(".openanalyst").is_dir());
        assert!(root.join(".openanalyst.json").is_file());
        assert!(root.join("OPENANALYST.md").is_file());
        assert_eq!(
            fs::read_to_string(root.join(".openanalyst.json")).expect("read openanalyst json"),
            concat!(
                "{\n",
                "  \"permissions\": {\n",
                "    \"defaultMode\": \"dontAsk\"\n",
                "  }\n",
                "}\n",
            )
        );
        let gitignore = fs::read_to_string(root.join(".gitignore")).expect("read gitignore");
        assert!(gitignore.contains(".openanalyst/settings.local.json"));
        assert!(gitignore.contains(".openanalyst/sessions/"));
        assert!(gitignore.contains(".openanalyst/.env"));
        assert!(gitignore.contains("OPENANALYST.local.md"));
        let oa_md = fs::read_to_string(root.join("OPENANALYST.md")).expect("read openanalyst md");
        assert!(oa_md.contains("Languages: Rust."));
        assert!(oa_md.contains("cargo clippy --workspace --all-targets -- -D warnings"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn initialize_repo_is_idempotent_and_preserves_existing_files() {
        let root = temp_dir();
        fs::create_dir_all(&root).expect("create root");
        fs::write(root.join("OPENANALYST.md"), "custom guidance\n").expect("write existing openanalyst md");
        fs::write(root.join(".gitignore"), ".openanalyst/settings.local.json\n.openanalyst/sessions/\n.openanalyst/.env\nOPENANALYST.local.md\n").expect("write gitignore");

        let first = initialize_repo(&root).expect("first init should succeed");
        assert!(first
            .render()
            .contains("OPENANALYST.md     skipped (already exists)"));
        let second = initialize_repo(&root).expect("second init should succeed");
        let second_rendered = second.render();
        assert!(second_rendered.contains(".openanalyst/      skipped (already exists)"));
        assert!(second_rendered.contains(".openanalyst.json  skipped (already exists)"));
        assert!(second_rendered.contains(".gitignore         skipped (already exists)"));
        assert!(second_rendered.contains("OPENANALYST.md     skipped (already exists)"));
        assert_eq!(
            fs::read_to_string(root.join("OPENANALYST.md")).expect("read existing openanalyst md"),
            "custom guidance\n"
        );
        let gitignore = fs::read_to_string(root.join(".gitignore")).expect("read gitignore");
        assert_eq!(gitignore.matches(".openanalyst/settings.local.json").count(), 1);
        assert_eq!(gitignore.matches(".openanalyst/sessions/").count(), 1);
        assert_eq!(gitignore.matches(".openanalyst/.env").count(), 1);

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn render_init_template_mentions_detected_python_and_nextjs_markers() {
        let root = temp_dir();
        fs::create_dir_all(&root).expect("create root");
        fs::write(root.join("pyproject.toml"), "[project]\nname = \"demo\"\n")
            .expect("write pyproject");
        fs::write(
            root.join("package.json"),
            r#"{"dependencies":{"next":"14.0.0","react":"18.0.0"},"devDependencies":{"typescript":"5.0.0"}}"#,
        )
        .expect("write package json");

        let rendered = render_init_openanalyst_md(Path::new(&root));
        assert!(rendered.contains("Languages: Python, TypeScript."));
        assert!(rendered.contains("Frameworks/tooling markers: Next.js, React."));
        assert!(rendered.contains("pyproject.toml"));
        assert!(rendered.contains("Next.js detected"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }
}
