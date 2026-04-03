//! Tool output masking — redacts secrets and sensitive data before sending to LLM.
//!
//! Scans tool outputs for patterns that look like API keys, tokens, passwords,
//! Scans tool outputs for patterns that look like API keys, tokens, passwords,
//! and replaces them with `[REDACTED]` placeholders.

use std::borrow::Cow;

use regex::Regex;
use std::sync::LazyLock;

// ── Patterns ─────────────────────────────────────────────────────────────────

/// Common secret patterns to redact from tool outputs.
static SECRET_PATTERNS: LazyLock<Vec<SecretPattern>> = LazyLock::new(|| {
    vec![
        // API keys with known prefixes
        SecretPattern::new("OpenAI API Key", r"sk-[A-Za-z0-9\-]{20,}"),
        SecretPattern::new("Anthropic API Key", r"sk-ant-[A-Za-z0-9\-]{20,}"),
        SecretPattern::new("OpenAnalyst API Key", r"oa_[A-Za-z0-9]{16,}"),
        SecretPattern::new("GitHub Token", r"gh[ps]_[A-Za-z0-9]{36,}"),
        SecretPattern::new("GitHub Fine-Grained Token", r"github_pat_[A-Za-z0-9_]{40,}"),
        SecretPattern::new("AWS Access Key", r"AKIA[A-Z0-9]{16}"),
        SecretPattern::new("AWS Secret Key", r"(?i)aws_secret_access_key\s*[=:]\s*[A-Za-z0-9/+=]{40}"),
        SecretPattern::new("Google API Key", r"AIza[A-Za-z0-9\-_]{35}"),
        SecretPattern::new("Slack Token", r"xox[bprs]-[A-Za-z0-9\-]{10,}"),
        SecretPattern::new("Stripe Key", r"[sr]k_(live|test)_[A-Za-z0-9]{20,}"),
        SecretPattern::new("Twilio Key", r"SK[a-f0-9]{32}"),
        SecretPattern::new("SendGrid Key", r"SG\.[A-Za-z0-9\-_]{22,}\.[A-Za-z0-9\-_]{43,}"),
        SecretPattern::new("Mailgun Key", r"key-[a-f0-9]{32}"),
        SecretPattern::new("JWT Token", r"eyJ[A-Za-z0-9\-_]+\.eyJ[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+"),
        SecretPattern::new("Bearer Token", r"(?i)bearer\s+[A-Za-z0-9\-_.~+/]+=*"),
        SecretPattern::new("Private Key Block", r"-----BEGIN (?:RSA |EC |DSA )?PRIVATE KEY-----"),
        // Generic patterns
        SecretPattern::new("Hex Secret (32+)", r#"(?i)(?:secret|token|password|api[_-]?key)\s*[=:]\s*['"]?[a-f0-9]{32,}['"]?"#),
        SecretPattern::new("Base64 Secret", r#"(?i)(?:secret|token|password|api[_-]?key)\s*[=:]\s*['"]?[A-Za-z0-9+/]{40,}={0,2}['"]?"#),
        // Connection strings
        SecretPattern::new("Database URL", r#"(?i)(?:postgres|mysql|mongodb|redis)://[^\s'"]+:[^\s'"]+@"#),
    ]
});

/// Environment variable names that should have their values redacted.
static SENSITIVE_ENV_VARS: &[&str] = &[
    "OPENANALYST_API_KEY", "ANTHROPIC_API_KEY", "OPENAI_API_KEY",
    "GEMINI_API_KEY", "XAI_API_KEY", "OPENROUTER_API_KEY",
    "BEDROCK_API_KEY", "AWS_SECRET_ACCESS_KEY", "AWS_SESSION_TOKEN",
    "STABILITY_API_KEY", "OPENANALYST_AUTH_TOKEN",
    "GITHUB_TOKEN", "GH_TOKEN", "SLACK_TOKEN",
    "DATABASE_URL", "REDIS_URL", "MONGO_URI",
];

// ── Types ────────────────────────────────────────────────────────────────────

struct SecretPattern {
    #[allow(dead_code)]
    name: &'static str,
    regex: Regex,
}

impl SecretPattern {
    fn new(name: &'static str, pattern: &str) -> Self {
        Self {
            name,
            regex: Regex::new(pattern).expect("invalid secret pattern regex"),
        }
    }
}

/// Statistics about what was redacted.
#[derive(Debug, Clone, Default)]
pub struct MaskingStats {
    pub secrets_redacted: u32,
    pub env_vars_redacted: u32,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Mask secrets in a tool output string.
///
/// Returns the masked string and stats about what was redacted.
pub fn mask_tool_output(output: &str) -> (Cow<'_, str>, MaskingStats) {
    if output.is_empty() {
        return (Cow::Borrowed(output), MaskingStats::default());
    }

    let mut result = output.to_string();
    let mut stats = MaskingStats::default();

    // Phase 1: Redact known secret patterns
    for pattern in SECRET_PATTERNS.iter() {
        let count_before = stats.secrets_redacted;
        result = pattern.regex.replace_all(&result, "[REDACTED]").to_string();
        // Count replacements (approximate — regex replace_all doesn't return count)
        if result.contains("[REDACTED]") && count_before == stats.secrets_redacted {
            stats.secrets_redacted += 1;
        }
    }

    // Phase 2: Redact environment variable values
    for var_name in SENSITIVE_ENV_VARS {
        if let Ok(val) = std::env::var(var_name) {
            if !val.is_empty() && val.len() >= 8 && result.contains(&val) {
                result = result.replace(&val, &format!("[REDACTED:{var_name}]"));
                stats.env_vars_redacted += 1;
            }
        }
    }

    if stats.secrets_redacted > 0 || stats.env_vars_redacted > 0 {
        (Cow::Owned(result), stats)
    } else {
        (Cow::Borrowed(output), stats)
    }
}

/// Quick check if output likely contains secrets (without full regex scan).
/// Use this as a fast pre-filter before calling `mask_tool_output`.
pub fn likely_contains_secrets(output: &str) -> bool {
    // Quick keyword scan
    let lower = output.to_ascii_lowercase();
    lower.contains("sk-") ||
    lower.contains("sk_") ||
    lower.contains("api_key") ||
    lower.contains("api-key") ||
    lower.contains("token") ||
    lower.contains("secret") ||
    lower.contains("password") ||
    lower.contains("bearer") ||
    lower.contains("private key") ||
    lower.contains("akia") ||
    lower.contains("ghp_") ||
    lower.contains("ghs_") ||
    lower.contains("eyj")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_openai_key() {
        let input = "Found key: sk-proj-1234567890abcdefABCDEF in config";
        let (output, stats) = mask_tool_output(input);
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("sk-proj-"));
        assert!(stats.secrets_redacted > 0);
    }

    #[test]
    fn redact_github_token() {
        let input = "export GITHUB_TOKEN=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefgh1234";
        let (output, _) = mask_tool_output(input);
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("ghp_"));
    }

    #[test]
    fn redact_jwt() {
        let input = "Authorization: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let (output, _) = mask_tool_output(input);
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn redact_private_key() {
        let input = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA0Z3VS5JJcds3xfn/ygWyF0";
        let (output, _) = mask_tool_output(input);
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn no_redaction_for_normal_text() {
        let input = "Hello world, this is a normal output with no secrets.";
        let (output, stats) = mask_tool_output(input);
        assert_eq!(output.as_ref(), input);
        assert_eq!(stats.secrets_redacted, 0);
    }

    #[test]
    fn likely_contains_secrets_quick_check() {
        assert!(likely_contains_secrets("Found sk-proj-12345"));
        assert!(likely_contains_secrets("Bearer eyJhbGciOi"));
        assert!(!likely_contains_secrets("Hello world"));
    }

    #[test]
    fn redact_database_url() {
        let input = "DATABASE_URL=postgres://admin:supersecret123@db.example.com:5432/mydb";
        let (output, _) = mask_tool_output(input);
        assert!(output.contains("[REDACTED]"));
    }
}
