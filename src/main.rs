mod cli;
mod models;
mod storage;
mod tools;
mod agents;
mod config;
mod sandbox;
mod reporting;
mod vulns;
mod llm;
mod workflow;

use clap::Parser;
use cli::{Cli, Commands};
use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "strike=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { target, env, auth, roe, output_dir } => {
            cli::handle_init(target, env, auth, roe, output_dir).await
        }
        Commands::Run { profile, focus, workers, resume, dry_run, no_exploit, max_depth, rate_limit, timeout, output, ci } => {
            cli::handle_run(profile, focus, workers, resume, dry_run, no_exploit, max_depth, rate_limit, timeout, output, ci).await
        }
        Commands::Recon { target, subdomains, ports, tech_detect, api_schema, crawl_depth } => {
            cli::handle_recon(target, subdomains, ports, tech_detect, api_schema, crawl_depth).await
        }
        Commands::Scan { endpoint, class, method, param, auth_session, validate } => {
            cli::handle_scan(endpoint, class, method, param, auth_session, validate).await
        }
        Commands::Validate { finding_id, replay, env } => {
            cli::handle_validate(finding_id, replay, env).await
        }
        Commands::Retest { finding_id, expect_fixed, update_status } => {
            cli::handle_retest(finding_id, expect_fixed, update_status).await
        }
        Commands::Report { run_id, format, severity, confirmed_only, include_evidence, standards } => {
            cli::handle_report(run_id, format, severity, confirmed_only, include_evidence, standards).await
        }
        Commands::Ci { config, fail_on, block_routes, upload_results } => {
            cli::handle_ci(config, fail_on, block_routes, upload_results).await
        }
        Commands::Agent { agent, target, llm, model, headless } => {
            cli::handle_agent(agent, target, llm, model, headless).await
        }
        Commands::Status { run_id } => {
            cli::handle_status(run_id).await
        }
        Commands::Findings { run_id, severity, status, class, format } => {
            cli::handle_findings(run_id, severity, status, class, format).await
        }
        Commands::Config { set, get, show, validate } => {
            cli::handle_config(set, get, show, validate).await
        }
        Commands::Benchmark { suite, report, compare_baseline } => {
            cli::handle_benchmark(suite, report, compare_baseline).await
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
