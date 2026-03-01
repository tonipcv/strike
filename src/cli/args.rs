use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "strike")]
#[command(version = "0.1.0")]
#[command(about = "Evidence-first CLI security validation platform", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(long, global = true, help = "Enable verbose logging")]
    pub verbose: bool,

    #[arg(long, global = true, help = "Output format: json, text")]
    pub format: Option<String>,

    #[arg(long, global = true, help = "Allow private IPs and localhost (use with caution)")]
    pub allow_private: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Initialize a new engagement workspace")]
    Init {
        #[arg(long, help = "Target URL or repository")]
        target: String,

        #[arg(long, default_value = "local", help = "Environment: staging, sandbox, local")]
        env: String,

        #[arg(long, help = "Authentication config file path")]
        auth: Option<String>,

        #[arg(long, help = "Rules of Engagement file path")]
        roe: Option<String>,

        #[arg(long, default_value = ".strike", help = "Output directory")]
        output_dir: String,
    },

    #[command(about = "Execute a full validation pipeline")]
    Run {
        #[arg(long, default_value = "full", help = "Profile: web, api, code, full")]
        profile: String,

        #[arg(long, help = "Focus on specific vulnerability classes (comma-separated)")]
        focus: Option<String>,

        #[arg(long, default_value = "16", help = "Number of concurrent workers")]
        workers: u32,

        #[arg(long, help = "Resume from a previous run ID")]
        resume: Option<String>,

        #[arg(long, help = "Dry run mode - no actual exploitation")]
        dry_run: bool,

        #[arg(long, help = "Disable exploitation, validation only")]
        no_exploit: bool,

        #[arg(long, default_value = "10", help = "Maximum crawl depth")]
        max_depth: u32,

        #[arg(long, default_value = "50", help = "Rate limit (requests per second)")]
        rate_limit: u32,

        #[arg(long, default_value = "300", help = "Timeout in seconds")]
        timeout: u32,

        #[arg(long, default_value = "json", help = "Output format: json, md, sarif, html, pdf")]
        output: String,

        #[arg(long, help = "CI mode - exit non-zero on policy violations")]
        ci: bool,
    },

    #[command(about = "Standalone reconnaissance phase")]
    Recon {
        #[arg(long, help = "Target URL")]
        target: String,

        #[arg(long, help = "Enable subdomain enumeration")]
        subdomains: bool,

        #[arg(long, help = "Enable port scanning")]
        ports: bool,

        #[arg(long, help = "Enable technology detection")]
        tech_detect: bool,

        #[arg(long, help = "API schema type: openapi, graphql, auto")]
        api_schema: Option<String>,

        #[arg(long, default_value = "5", help = "Crawl depth")]
        crawl_depth: u32,
    },

    #[command(about = "Run targeted vulnerability scan")]
    Scan {
        #[arg(long, help = "Target endpoint URL")]
        endpoint: String,

        #[arg(long, help = "Vulnerability class to scan for")]
        class: Option<String>,

        #[arg(long, default_value = "GET", help = "HTTP method")]
        method: String,

        #[arg(long, help = "Parameter name to test")]
        param: Option<String>,

        #[arg(long, help = "Authentication session file")]
        auth_session: Option<String>,

        #[arg(long, help = "Validate findings immediately")]
        validate: bool,
    },

    #[command(about = "Re-run validation on a specific finding")]
    Validate {
        #[arg(long, help = "Finding ID to validate")]
        finding_id: String,

        #[arg(long, help = "Replay the original request")]
        replay: bool,

        #[arg(long, help = "Target environment")]
        env: Option<String>,
    },

    #[command(about = "Retest a previously confirmed finding")]
    Retest {
        #[arg(long, help = "Finding ID to retest")]
        finding_id: String,

        #[arg(long, help = "Expect the finding to be fixed")]
        expect_fixed: bool,

        #[arg(long, help = "Update finding status based on result")]
        update_status: bool,
    },

    #[command(about = "Generate report from findings store")]
    Report {
        #[arg(long, help = "Run ID to generate report for")]
        run_id: Option<String>,

        #[arg(long, default_value = "json", help = "Format: json, md, sarif, html, pdf")]
        format: String,

        #[arg(long, help = "Filter by severity: critical, high, medium, low, info")]
        severity: Option<String>,

        #[arg(long, help = "Only include confirmed findings")]
        confirmed_only: bool,

        #[arg(long, help = "Include full evidence bundles")]
        include_evidence: bool,

        #[arg(long, help = "Standards mapping: owasp, asvs, cvss, all")]
        standards: Option<String>,
    },

    #[command(about = "CI/CD mode with policy gates")]
    Ci {
        #[arg(long, help = "CI configuration file")]
        config: String,

        #[arg(long, help = "Fail on severity level: critical, high, medium")]
        fail_on: Option<String>,

        #[arg(long, help = "Block routes (comma-separated patterns)")]
        block_routes: Option<String>,

        #[arg(long, help = "Upload results to external system")]
        upload_results: bool,
    },

    #[command(about = "Start specific agent in interactive or headless mode")]
    Agent {
        #[arg(long, help = "Agent type: recon, auth, hypothesis, validation, evidence, remediation, retest")]
        agent: String,

        #[arg(long, help = "Target URL")]
        target: String,

        #[arg(long, default_value = "claude", help = "LLM provider: claude, gpt4, ollama")]
        llm: String,

        #[arg(long, help = "Model ID")]
        model: Option<String>,

        #[arg(long, help = "Run in headless mode")]
        headless: bool,
    },

    #[command(about = "Show current run status")]
    Status {
        #[arg(long, help = "Run ID to check status for")]
        run_id: Option<String>,
    },

    #[command(about = "Query and filter findings")]
    Findings {
        #[arg(long, help = "Filter by run ID")]
        run_id: Option<String>,

        #[arg(long, help = "Filter by severity")]
        severity: Option<String>,

        #[arg(long, help = "Filter by status")]
        status: Option<String>,

        #[arg(long, help = "Filter by vulnerability class")]
        class: Option<String>,

        #[arg(long, default_value = "table", help = "Output format: table, json, csv")]
        format: String,
    },

    #[command(about = "Manage workspace configuration")]
    Config {
        #[arg(long, help = "Set configuration key=value")]
        set: Option<String>,

        #[arg(long, help = "Get configuration value by key")]
        get: Option<String>,

        #[arg(long, help = "Show all configuration")]
        show: bool,

        #[arg(long, help = "Validate configuration")]
        validate: bool,
    },

    #[command(about = "Run benchmark against test targets")]
    Benchmark {
        #[arg(long, default_value = "juice-shop", help = "Benchmark suite: juice-shop, webgoat, dvwa, custom")]
        suite: String,

        #[arg(long, help = "Generate benchmark report")]
        report: bool,

        #[arg(long, help = "Compare against baseline")]
        compare_baseline: bool,
    },
}
