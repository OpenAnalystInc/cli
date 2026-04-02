# Contributing to OpenAnalyst CLI

Thank you for your interest in contributing to OpenAnalyst CLI. This document explains how to get involved.

## Getting Started

1. **Fork** the repository on GitHub
2. **Clone** your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/openanalyst-cli.git
   cd openanalyst-cli
   ```
3. **Create a branch** for your change:
   ```bash
   git checkout -b feature/your-feature-name
   ```
4. **Build and test**:
   ```bash
   cd rust
   cargo build
   cargo test --workspace
   ```

## Development Workflow

### No Direct Commits to `main`

All changes must go through a **pull request**. Direct pushes to `main` are blocked.

1. Make your changes on a feature branch
2. Ensure `cargo check --workspace` passes with no errors
3. Ensure `cargo test --workspace` passes
4. Run `cargo clippy --workspace` and fix any warnings
5. Run `cargo fmt --all` to format code
6. Push your branch and open a pull request

### Code Style

- **Language:** Rust (edition 2021)
- **Formatting:** `cargo fmt` — run before every commit
- **Linting:** `cargo clippy` — all warnings must be resolved
- **Unsafe code:** Forbidden (`#[forbid(unsafe_code)]`)
- **Tests:** Add tests for new functionality

### Commit Messages

Use clear, descriptive commit messages:

```
Add /doctor command for installation diagnostics

- Checks provider API keys
- Validates MCP server configuration
- Reports workspace status
```

- Start with a verb in imperative form (Add, Fix, Update, Remove)
- First line under 72 characters
- Body explains what and why, not how

### Pull Request Process

1. Fill out the PR template completely
2. Link any related issues
3. Ensure CI passes (if configured)
4. Request review from `@AnitChaudhry`
5. Address review feedback
6. Maintainer will merge when approved

## Project Structure

```
rust/crates/
├── api/                 # Multi-provider API client
├── commands/            # Slash command definitions
├── events/              # TUI event types
├── orchestrator/        # Multi-agent engine
├── tui/                 # Ratatui TUI application
├── tui-widgets/         # Custom TUI widgets
├── runtime/             # Conversation engine
├── tools/               # Built-in tool implementations
├── plugins/             # Plugin system
├── openanalyst-cli/     # Binary entry point
├── openanalyst-agent/   # Headless agent runner
├── server/              # HTTP/SSE server
├── lsp/                 # LSP integration
└── compat-harness/      # Manifest extraction
```

## Reporting Issues

- Use the **Bug Report** template for bugs
- Use the **Feature Request** template for new ideas
- For security vulnerabilities, see [SECURITY.md](SECURITY.md)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
