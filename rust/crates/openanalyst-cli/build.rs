//! Build script — inject version from git tag at compile time.
//!
//! Priority:
//!   1. `OA_VERSION` env var (set by CI/release scripts)
//!   2. `git describe --tags` (e.g., "v1.0.100" → "1.0.100")
//!   3. Falls back to CARGO_PKG_VERSION from Cargo.toml

use std::process::Command;

fn main() {
    // Re-run if git HEAD changes (new commit/tag)
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-env-changed=OA_VERSION");

    let version = std::env::var("OA_VERSION")
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(git_version)
        .unwrap_or_else(|| std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".into()));

    println!("cargo:rustc-env=OA_BUILD_VERSION={version}");
}

fn git_version() -> Option<String> {
    let output = Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let tag = String::from_utf8(output.stdout).ok()?;
    let tag = tag.trim();
    if tag.is_empty() {
        return None;
    }

    // Strip leading 'v' (e.g., "v1.0.100" → "1.0.100")
    Some(tag.trim_start_matches('v').to_string())
}
