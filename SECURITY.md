# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in OpenAnalyst CLI, please report it responsibly.

**Do NOT open a public GitHub issue for security vulnerabilities.**

Instead, email: **security@openanalyst.com**

### What to include

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Any suggested fixes (optional)

### What counts as a security issue

- Remote code execution
- Credential exposure (API keys, tokens)
- Path traversal or sandbox escape
- Injection attacks (command injection, etc.)
- Authentication/authorization bypass
- Data exfiltration through tool misuse

### Response Timeline

- **Acknowledgment:** Within 48 hours
- **Assessment:** Within 7 days
- **Fix:** Depends on severity (critical: ASAP, high: 14 days, medium: 30 days)
- **Disclosure:** Coordinated with reporter

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.0.x   | Yes       |

## Security Best Practices for Users

- Never commit API keys to version control
- Use `openanalyst login` for credential management (stored in `~/.openanalyst/credentials.json`)
- Review tool permissions before granting `danger-full-access` mode
- Use `read-only` or `workspace-write` permission modes for untrusted projects
