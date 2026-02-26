# Strike

**Evidence-first CLI security validation platform. Break it before they do — with proof.**

[![License](https://img.shields.io/badge/license-BSL--1.1-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/strike.svg)](https://crates.io/crates/strike)
[![Documentation](https://docs.rs/strike/badge.svg)](https://docs.rs/strike)

Strike is a Rust-powered CLI security validation platform designed for penetration testers, red team operators, AppSec engineers, and security researchers. It provides evidence-first, reproducible security testing with standards-mapped findings.

## Features

- **Evidence-First**: Every finding includes validated proof-of-concept with full HTTP traces
- **Reproducible**: Deterministic runs with checkpoint support and replay capability
- **Standards-Mapped**: Automatic mapping to OWASP Top 10, API Security Top 10, WSTG, ASVS, and CVSS v4.0
- **High Performance**: Built in Rust with async/await for parallel execution (up to 64 concurrent workers)
- **CI/CD Native**: SARIF output, policy gates, and exit codes for pipeline integration
- **Multi-Agent Architecture**: Specialized agents for recon, auth, validation, evidence, and reporting
- **Safety by Default**: Production environment blocks, scope validation, and ROE enforcement

## Installation

### From crates.io

```bash
cargo install strike
```

### From source

```bash
git clone https://github.com/xaseai/strike
cd strike
cargo build --release
```

## Quick Start

### 1. Initialize a workspace

```bash
strike init --target https://staging.example.com --env staging
```

### 2. Run reconnaissance

```bash
strike recon --target https://staging.example.com --subdomains --ports --tech-detect
```

### 3. Execute full validation pipeline

```bash
strike run --profile full --workers 16 --rate-limit 50
```

### 4. View findings

```bash
strike findings --severity critical --status confirmed --format table
```

### 5. Generate reports

```bash
strike report --format sarif --confirmed-only --include-evidence
```

## CLI Commands

### Core Commands

- `strike init` - Initialize a new engagement workspace
- `strike run` - Execute full validation pipeline
- `strike recon` - Standalone reconnaissance phase
- `strike scan` - Targeted vulnerability scan
- `strike validate` - Re-validate a specific finding
- `strike retest` - Retest after remediation

### Management Commands

- `strike status` - Show current run status
- `strike findings` - Query and filter findings
- `strike report` - Generate reports (JSON, Markdown, SARIF, HTML, PDF)
- `strike config` - Manage workspace configuration
- `strike benchmark` - Run against test targets (OWASP Juice Shop, WebGoat)

### CI/CD Integration

- `strike ci` - CI/CD mode with policy gates

## Vulnerability Classes Supported

### Access Control
- IDOR/BOLA
- BFLA (Broken Function Level Authorization)
- Privilege Escalation
- Path Traversal
- Mass Assignment

### Injection
- SQL Injection
- NoSQL Injection
- OS Command Injection
- SSTI (Server-Side Template Injection)
- XPath/LDAP Injection

### Authentication & Session
- Broken Authentication
- Session Fixation
- Token Forgery
- JWT Weaknesses
- OAuth2 Misconfigurations
- 2FA Bypass

### Client-Side
- XSS (Reflected, Stored, DOM)
- CSRF
- Clickjacking
- Open Redirect

### Server-Side
- SSRF
- XXE
- Deserialization
- File Upload Abuse
- Race Conditions

### API-Specific
- Mass Data Exposure
- Unrestricted Resource Consumption
- Security Misconfiguration
- Improper Asset Management

## Configuration

Strike uses a `strike.toml` configuration file:

```toml
target = "https://staging.example.com"
env = "staging"
profile = "full"
workers = 16
rate_limit = 50

[llm]
provider = "anthropic"
model = "claude-sonnet-4-6"
max_tokens_per_agent = 4096

[sandbox]
driver = "docker"
network_allowlist = ["staging.example.com"]

[output]
dir = "./.strike/runs"
formats = ["json", "md", "sarif"]
```

## Output Formats

- **JSON**: Machine-readable findings bundle
- **Markdown**: Developer-friendly report
- **SARIF**: CI/CD integration (GitHub Security, GitLab, etc.)
- **HTML**: Standalone report
- **PDF**: Audit-ready documentation

## Evidence Bundle Schema

Each validated finding includes:

- **Proof of Concept**: Full HTTP request/response traces
- **CVSS v4.0 Score**: Automated scoring with environmental tuning
- **Standards Mapping**: OWASP, ASVS, CWE references
- **Remediation Guidance**: Developer-ready fix suggestions
- **Retest History**: Track fix validation over time
- **Authorization**: ROE reference and approval metadata

## Safety & Ethics

Strike enforces mandatory safety guardrails:

- **Scope Validation**: All targets must be explicitly authorized
- **Environment Protection**: Production environments blocked by default
- **Rate Limiting**: Configurable request throttling
- **ROE Enforcement**: Rules of Engagement validated before execution
- **Evidence Sanitization**: Automatic PII/credential redaction

**Legal Notice**: Strike is designed exclusively for authorized security testing. Use only on systems you own or have explicit written permission to test.

## Architecture

- **Runtime**: Tokio async runtime for high concurrency
- **Storage**: SQLite (local) or PostgreSQL (team mode)
- **HTTP Client**: reqwest with rustls (no OpenSSL dependency)
- **Sandbox**: Docker isolation with network allowlisting
- **Observability**: OpenTelemetry tracing and structured logging

## Performance Targets

- **Scan Startup**: < 100ms cold start
- **Concurrent Workers**: Up to 64 parallel tasks
- **Memory Footprint**: < 30MB RSS idle
- **HTTP Throughput**: 10,000+ req/s
- **Report Generation**: < 2s for full evidence bundle

## Roadmap

### Phase 1 (Current - v0.1.0)
- ✅ Core CLI framework
- ✅ Multi-agent architecture
- ✅ SQLite storage
- ✅ Evidence bundle schema
- ✅ CVSS v4.0 scoring
- ✅ JSON/Markdown/SARIF reports

### Phase 2 (v0.2.0)
- Durable workflow state with checkpointing
- Full WSTG + PTES mapping
- LLM-powered hypothesis generation
- Root cause analysis
- PostgreSQL team mode

### Phase 3 (v0.3.0)
- Human-in-the-loop review workflow
- RBAC for team workspaces
- Air-gapped deployment mode
- Comprehensive ASVS coverage

## Contributing

Contributions are welcome! Please read our contributing guidelines and code of conduct.

## License

Strike is licensed under the Business Source License 1.1 (BSL-1.1). See [LICENSE](LICENSE) for details.

## Support

- **Documentation**: https://docs.strike.dev
- **Issues**: https://github.com/xaseai/strike/issues
- **Discussions**: https://github.com/xaseai/strike/discussions

## Acknowledgments

Strike follows OWASP, PTES, and ASVS best practices. Built with Rust for performance and safety.

---

**Strike** - Evidence-first security validation. Break it before they do.
