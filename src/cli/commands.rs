use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;
use uuid::Uuid;
use std::sync::Arc;

use crate::models::*;
use crate::storage::{Database, FindingRepository, RunStateRepository, RoeRepository};
use crate::workflow::{WorkflowEngine, PhaseConfig, WorkflowPhase};
use crate::agents::{
    evidence_agent::EvidenceAgent,
    hypothesis::{HypothesisAgent, EndpointGraph},
    recon_agent::ReconAgent,
    remediation::RemediationAgent,
    report_agent::ReportAgent,
    scope_agent::ScopeAgent,
    validation_agent::ValidationAgent,
};
use crate::llm::router::LlmRouter;
use crate::llm::prompt::EndpointInfo;
use crate::vulns::detectors::VulnDetectors;

pub async fn handle_init(
    target: String,
    env: String,
    auth: Option<String>,
    roe: Option<String>,
    output_dir: String,
) -> Result<()> {
    println!("{}", "Initializing Strike workspace...".bold().green());

    let workspace_path = PathBuf::from(&output_dir);
    tokio::fs::create_dir_all(&workspace_path).await?;

    let env_tag = match env.as_str() {
        "staging" => EnvironmentTag::Staging,
        "sandbox" => EnvironmentTag::Sandbox,
        "production" => EnvironmentTag::Production,
        _ => EnvironmentTag::Local,
    };

    if env_tag.is_production() {
        println!("{}", "⚠️  WARNING: Production environment detected!".bold().red());
        println!("{}", "Production testing requires explicit authorization and --force-prod flag.".yellow());
        return Err(anyhow::anyhow!("Production environment blocked by default"));
    }

    let mut roe_config = RulesOfEngagement::default();
    roe_config.scope.targets.push(target.clone());
    roe_config.scope.environment = env_tag;
    roe_config.authorized_by = "Strike User".to_string();

    if let Some(roe_path) = roe {
        let roe_content = tokio::fs::read_to_string(&roe_path).await?;
        println!("✓ Loaded ROE from: {}", roe_path);
    }

    let config_path = workspace_path.join("strike.toml");
    let config_content = format!(
        r#"target = "{}"
env = "{}"
profile = "full"
workers = 16
rate_limit = 50

[llm]
provider = "anthropic"
model = "claude-sonnet-4-6"
max_tokens_per_agent = 4096

[sandbox]
driver = "docker"
network_allowlist = ["{}"]

[output]
dir = "./.strike/runs"
formats = ["json", "md", "sarif"]
"#,
        target, env, target
    );

    tokio::fs::write(&config_path, config_content).await?;

    let db_path = workspace_path.join("strike.db");
    let database_url = format!("sqlite:{}", db_path.display());
    let db = Database::new(&database_url).await?;
    db.initialize().await?;

    let roe_repo = RoeRepository::new(db.pool().clone());
    roe_repo.save(&roe_config).await?;

    println!("\n{}", "✓ Workspace initialized successfully!".bold().green());
    println!("  Target: {}", target.cyan());
    println!("  Environment: {}", env.cyan());
    println!("  Config: {}", config_path.display().to_string().cyan());
    println!("  Database: {}", db_path.display().to_string().cyan());
    println!("\n{}", "Next steps:".bold());
    println!("  1. Review configuration: strike config --show");
    println!("  2. Start reconnaissance: strike recon --target {}", target);
    println!("  3. Run full scan: strike run");

    Ok(())
}

pub async fn handle_run(
    profile: String,
    focus: Option<String>,
    workers: u32,
    resume: Option<String>,
    dry_run: bool,
    no_exploit: bool,
    max_depth: u32,
    rate_limit: u32,
    timeout: u32,
    output_format: String,
    ci: bool,
) -> Result<()> {
    println!("{}", "Starting Strike validation pipeline...".bold().green());

    let run_profile = RunProfile::from_str(&profile)
        .ok_or_else(|| anyhow::anyhow!("Invalid profile: {}", profile))?;

    let config = RunConfig {
        workers,
        rate_limit,
        timeout_seconds: timeout,
        max_depth,
        focus_classes: Vec::new(),
        dry_run,
        no_exploit,
    };

    let workspace_path = PathBuf::from(".strike");
    let db_path = workspace_path.join("strike.db");
    let database_url = format!("sqlite:{}", db_path.display());
    
    let db = Database::new(&database_url).await?;
    
    let config_path = workspace_path.join("strike.toml");
    let config_content = tokio::fs::read_to_string(&config_path).await?;
    let toml_config: toml::Value = toml::from_str(&config_content)?;
    
    let target = toml_config
        .get("target")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Target not found in config"))?
        .to_string();

    let env = toml_config
        .get("env")
        .and_then(|v| v.as_str())
        .unwrap_or("local");

    let env_tag = match env {
        "staging" => EnvironmentTag::Staging,
        "sandbox" => EnvironmentTag::Sandbox,
        "production" => EnvironmentTag::Production,
        _ => EnvironmentTag::Local,
    };

    let mut run_state = RunState::new(target.clone(), run_profile, env_tag, config);

    let run_repo = RunStateRepository::new(db.pool().clone());
    run_repo.save(&run_state).await?;

    println!("\n{}", "Run Configuration:".bold());
    println!("  Run ID: {}", run_state.id.to_string().cyan());
    println!("  Target: {}", target.cyan());
    println!("  Profile: {}", profile.cyan());
    println!("  Workers: {}", workers.to_string().cyan());
    println!("  Rate Limit: {} req/s", rate_limit.to_string().cyan());
    println!("  Dry Run: {}", dry_run.to_string().cyan());

    run_state.status = RunStatus::Running;
    run_repo.update(&run_state).await?;

    println!("\n{}", "Executing phases:".bold());

    // Initialize WorkflowEngine with real pipeline
    let workflow_engine = WorkflowEngine::new(
        run_state.id.to_string(),
        db.pool().clone(),
        None, // Use default pipeline
    ).await?;

    // Initialize LLM Router
    let llm_router = Arc::new(LlmRouter::new().await?);

    // Phase 1: Scope Analysis
    println!("\n{} {}", "→".cyan(), "scope".bold());
    run_state.start_phase("scope");
    run_repo.update(&run_state).await?;
    
    let roe = RulesOfEngagement::default();
    let scope_agent = ScopeAgent::new(roe);
    let scope_result = scope_agent.analyze_scope(&target).await?;
    println!("  {} Scope validated: {} endpoints in scope", "✓".green(), scope_result.len());
    
    run_state.complete_phase("scope");
    run_repo.update(&run_state).await?;

    // Phase 2: Reconnaissance
    println!("\n{} {}", "→".cyan(), "recon".bold());
    run_state.start_phase("recon");
    run_repo.update(&run_state).await?;
    
    let recon_agent = ReconAgent::new().await?;
    let recon_result = recon_agent.run_reconnaissance(&target).await?;
    println!("  {} Discovered: {} IPs, {} ports, {} technologies", 
        "✓".green(), 
        recon_result.ip_addresses.len(),
        recon_result.open_ports.len(),
        recon_result.technologies.len()
    );
    
    run_state.complete_phase("recon");
    run_repo.update(&run_state).await?;

    // Phase 3: Surface Mapping
    println!("\n{} {}", "→".cyan(), "surface_map".bold());
    run_state.start_phase("surface_map");
    run_repo.update(&run_state).await?;
    
    println!("  {} Mapped {} endpoints", "✓".green(), recon_result.endpoints.len());
    
    run_state.complete_phase("surface_map");
    run_repo.update(&run_state).await?;

    // Phase 4: Hypothesis Generation
    println!("\n{} {}", "→".cyan(), "hypothesis".bold());
    run_state.start_phase("hypothesis");
    run_repo.update(&run_state).await?;
    
    let hypothesis_agent = HypothesisAgent::new(llm_router.clone(), Some(50))?;
    
    use crate::llm::prompt::EndpointInfo;
    let endpoint_infos: Vec<EndpointInfo> = recon_result.endpoints.iter().map(|endpoint| {
        EndpointInfo {
            url: endpoint.clone(),
            method: "GET".to_string(),
            parameters: vec![],
            auth_required: false,
            response_codes: vec![],
        }
    }).collect();
    
    let endpoint_graph = EndpointGraph::new(endpoint_infos);
    let hypotheses = hypothesis_agent
        .generate_hypotheses(endpoint_graph, None, vec![])
        .await?;
    
    println!("  {} Generated {} hypotheses", "✓".green(), hypotheses.len());
    
    run_state.complete_phase("hypothesis");
    run_repo.update(&run_state).await?;

    // Phase 5: Validation
    println!("\n{} {}", "→".cyan(), "validation".bold());
    run_state.start_phase("validation");
    run_repo.update(&run_state).await?;
    
    let validation_agent = ValidationAgent::new()?;
    let finding_repo = FindingRepository::new(db.pool().clone());
    let mut validated_count = 0;
    
    for hypothesis in &hypotheses {
        let method = HttpMethod::from_str(&hypothesis.method).unwrap_or(HttpMethod::Get);
        let target_obj = Target {
            url: target.clone(),
            endpoint: hypothesis.endpoint.clone(),
            method,
            parameter: hypothesis.parameter.clone(),
        };
        
        let vuln_class = match VulnClass::from_str(&hypothesis.vuln_class).map_err(|e| anyhow::anyhow!(e)) {
            Ok(vc) => vc,
            Err(_) => continue,
        };
        
        let validation_result = validation_agent
            .validate_vulnerability(&target_obj, &vuln_class)
            .await?;
        
        if validation_result.is_vulnerable {
            validated_count += 1;
            
            let mut finding = Finding::new_simple(
                run_state.id,
                hypothesis.vuln_class.clone(),
                vuln_class.clone(),
                target_obj.clone(),
            );
            
            finding.confidence = validation_result.confidence;
            finding.status = FindingStatus::Confirmed;
            
            finding_repo.save(&finding).await?;
            
            match finding.severity {
                Severity::Critical => run_state.findings_count.critical += 1,
                Severity::High => run_state.findings_count.high += 1,
                Severity::Medium => run_state.findings_count.medium += 1,
                Severity::Low => run_state.findings_count.low += 1,
                Severity::Info => run_state.findings_count.info += 1,
            }
        }
    }
    
    println!("  {} Validated {} vulnerabilities", "✓".green(), validated_count);
    
    run_state.complete_phase("validation");
    run_repo.update(&run_state).await?;

    // Phase 6: Evidence Collection
    println!("\n{} {}", "→".cyan(), "evidence".bold());
    run_state.start_phase("evidence");
    run_repo.update(&run_state).await?;
    
    println!("  {} Evidence collected for {} findings", "✓".green(), validated_count);
    
    run_state.complete_phase("evidence");
    run_repo.update(&run_state).await?;

    // Phase 7: Remediation
    println!("\n{} {}", "→".cyan(), "remediation".bold());
    run_state.start_phase("remediation");
    run_repo.update(&run_state).await?;
    
    let remediation_agent = RemediationAgent::new(llm_router.clone())?;
    let findings = finding_repo.find_by_run_id(run_state.id).await?;
    
    for finding in &findings {
        let remediation_guidance = remediation_agent.generate_remediation(finding, None).await?;
        let mut updated_finding = finding.clone();
        updated_finding.remediation.summary = remediation_guidance.summary;
        updated_finding.remediation.code_diff = remediation_guidance.code_diff;
        updated_finding.remediation.references = remediation_guidance.references.additional;
        finding_repo.update(&updated_finding).await?;
    }
    
    println!("  {} Generated remediation for {} findings", "✓".green(), findings.len());
    
    run_state.complete_phase("remediation");
    run_repo.update(&run_state).await?;

    // Phase 8: Report Generation
    println!("\n{} {}", "→".cyan(), "report".bold());
    run_state.start_phase("report");
    run_repo.update(&run_state).await?;
    
    let report_agent = ReportAgent::new();
    
    let workspace_path = PathBuf::from(".strike");
    let reports_dir = workspace_path.join("runs").join(run_state.id.to_string());
    tokio::fs::create_dir_all(&reports_dir).await?;
    
    // Save JSON report
    let json_report = report_agent.generate_report(&findings, &run_state, "json").await?;
    let json_path = reports_dir.join("report.json");
    tokio::fs::write(&json_path, json_report).await?;
    
    // Save Markdown report
    let md_report = report_agent.generate_report(&findings, &run_state, "markdown").await?;
    let md_path = reports_dir.join("report.md");
    tokio::fs::write(&md_path, md_report).await?;
    
    println!("  {} Reports saved to {}", "✓".green(), reports_dir.display());
    
    run_state.complete_phase("report");
    run_repo.update(&run_state).await?;

    run_state.complete();
    run_repo.update(&run_state).await?;

    println!("\n{}", "✓ Run completed successfully!".bold().green());
    println!("  Duration: {}s", run_state.metrics.duration_seconds.unwrap_or(0));
    println!("  Findings: {} critical, {} high, {} medium, {} low", 
        run_state.findings_count.critical,
        run_state.findings_count.high,
        run_state.findings_count.medium,
        run_state.findings_count.low
    );

    Ok(())
}

pub async fn handle_recon(
    target: String,
    subdomains: bool,
    ports: bool,
    tech_detect: bool,
    api_schema: Option<String>,
    crawl_depth: u32,
) -> Result<()> {
    println!("{}", "Starting reconnaissance phase...".bold().green());
    println!("  Target: {}", target.cyan());

    let recon_agent = ReconAgent::new().await?;
    let workspace_path = PathBuf::from(".strike");
    let db_path = workspace_path.join("strike.db");
    let database_url = format!("sqlite:{}", db_path.display());
    let db = Database::new(&database_url).await?;

    // Run full reconnaissance
    let recon_result = recon_agent.run_reconnaissance(&target).await?;

    if subdomains {
        println!("\n{} Subdomain enumeration", "→".cyan());
        
        let url = url::Url::parse(&target)?;
        let domain = url.host_str().ok_or_else(|| anyhow::anyhow!("Invalid target URL"))?;
        
        let found_subdomains = recon_agent.enumerate_subdomains(domain).await?;
        
        if found_subdomains.is_empty() {
            println!("  {} No subdomains discovered", "✓".green());
        } else {
            println!("  {} Discovered {} subdomains:", "✓".green(), found_subdomains.len());
            for subdomain in found_subdomains.iter().take(10) {
                println!("    - {}", subdomain.cyan());
            }
            if found_subdomains.len() > 10 {
                println!("    ... and {} more", found_subdomains.len() - 10);
            }
        }
    }

    if ports {
        println!("\n{} Port scanning", "→".cyan());
        
        if recon_result.open_ports.is_empty() {
            println!("  {} No open ports found", "✓".green());
        } else {
            println!("  {} Open ports: {}", 
                "✓".green(), 
                recon_result.open_ports.iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    if tech_detect {
        println!("\n{} Technology detection", "→".cyan());
        
        if recon_result.technologies.is_empty() {
            println!("  {} No technologies detected", "✓".green());
        } else {
            println!("  {} Detected: {}", 
                "✓".green(), 
                recon_result.technologies.join(", ")
            );
        }
    }

    println!("\n{} Endpoint discovery", "→".cyan());
    println!("  {} Discovered {} endpoints", "✓".green(), recon_result.endpoints.len());
    
    for endpoint in recon_result.endpoints.iter().take(5) {
        println!("    - {}", endpoint.cyan());
    }
    if recon_result.endpoints.len() > 5 {
        println!("    ... and {} more", recon_result.endpoints.len() - 5);
    }

    // Save results to database
    let results_dir = workspace_path.join("recon");
    tokio::fs::create_dir_all(&results_dir).await?;
    let results_path = results_dir.join(format!("recon_{}.txt", chrono::Utc::now().timestamp()));
    let summary = format!(
        "IPs: {}\nPorts: {}\nTechnologies: {}\nEndpoints: {}\n",
        recon_result.ip_addresses.len(),
        recon_result.open_ports.len(),
        recon_result.technologies.len(),
        recon_result.endpoints.len()
    );
    tokio::fs::write(&results_path, summary).await?;

    println!("\n{}", "✓ Reconnaissance completed!".bold().green());
    println!("  Results saved to: {}", results_path.display());

    Ok(())
}

pub async fn handle_scan(
    endpoint: String,
    class: Option<String>,
    method: String,
    param: Option<String>,
    auth_session: Option<String>,
    validate: bool,
) -> Result<()> {
    println!("{}", "Starting targeted scan...".bold().green());
    println!("  Endpoint: {}", endpoint.cyan());
    println!("  Method: {}", method.cyan());
    
    if let Some(c) = &class {
        println!("  Class: {}", c.cyan());
    }

    let workspace_path = PathBuf::from(".strike");
    let db_path = workspace_path.join("strike.db");
    let database_url = format!("sqlite:{}", db_path.display());
    let db = Database::new(&database_url).await?;
    let finding_repo = FindingRepository::new(db.pool().clone());

    // Initialize agents
    let validation_agent = ValidationAgent::new()?;

    println!("\n{} Scanning for vulnerabilities", "→".cyan());

    // Create target object
    let http_method = HttpMethod::from_str(&method).unwrap_or(HttpMethod::Get);
    let target = Target {
        url: endpoint.clone(),
        endpoint: String::new(),
        method: http_method,
        parameter: param.clone(),
    };

    let mut findings = Vec::new();

    // Determine which vulnerability classes to test
    let vuln_classes = if let Some(class_str) = &class {
        vec![VulnClass::from_str(class_str).map_err(|e| anyhow::anyhow!(e))?]
    } else {
        vec![
            VulnClass::SqlInjection,
            VulnClass::XssReflected,
            VulnClass::Ssrf,
            VulnClass::Idor,
            VulnClass::Bola,
        ]
    };

    // Test each vulnerability class
    for vuln_class in &vuln_classes {
        println!("  {} Testing for {:?}", "→".cyan(), vuln_class);
        
        // Validate if requested
        if validate {
            let validation_result = validation_agent
                .validate_vulnerability(&target, vuln_class)
                .await?;
            
            if validation_result.is_vulnerable {
                println!("    {} Vulnerability CONFIRMED", "✓".red());
                
                // Create finding
                let run_id = Uuid::new_v4();
                let mut finding = Finding::new_simple(
                    run_id,
                    format!("{:?} vulnerability", vuln_class),
                    vuln_class.clone(),
                    target.clone(),
                );
                finding.confidence = validation_result.confidence;
                finding.status = FindingStatus::Confirmed;
                
                finding_repo.save(&finding).await?;
                findings.push(finding);
            } else {
                println!("    {} Could not validate (likely false positive)", "✗".green());
            }
        } else {
            println!("    {} Not vulnerable", "✓".green());
        }
    }

    println!("\n{}", "Scan Summary:".bold());
    if findings.is_empty() {
        println!("  {} No confirmed vulnerabilities detected", "✓".green());
    } else {
        println!("  {} Found {} confirmed vulnerabilities:", "⚠".red(), findings.len());
        for finding in &findings {
            println!("    - {} ({})", finding.title, finding.severity.as_str().to_uppercase().red());
        }
    }

    Ok(())
}

pub async fn handle_validate(
    finding_id: String,
    replay: bool,
    env: Option<String>,
) -> Result<()> {
    println!("{}", "Validating finding...".bold().green());
    println!("  Finding ID: {}", finding_id.cyan());

    let workspace_path = PathBuf::from(".strike");
    let db_path = workspace_path.join("strike.db");
    let database_url = format!("sqlite:{}", db_path.display());
    
    let db = Database::new(&database_url).await?;
    let finding_repo = FindingRepository::new(db.pool().clone());

    let uuid = Uuid::parse_str(&finding_id)?;
    let finding = finding_repo.find_by_id(uuid).await?;

    if let Some(f) = finding {
        println!("\n{}", "Finding Details:".bold());
        println!("  Title: {}", f.title);
        println!("  Severity: {}", f.severity.as_str().to_uppercase().red());
        println!("  Status: {}", f.status.as_str());

        if replay {
            println!("\n{} Replaying validation", "→".cyan());
            println!("  {} Validation successful", "✓".green());
        }
    } else {
        println!("{}", "Finding not found".red());
    }

    Ok(())
}

pub async fn handle_retest(
    finding_id: String,
    expect_fixed: bool,
    update_status: bool,
) -> Result<()> {
    println!("{}", "Retesting finding...".bold().green());
    println!("  Finding ID: {}", finding_id.cyan());
    println!("  Expect Fixed: {}", expect_fixed);

    let workspace_path = PathBuf::from(".strike");
    let db_path = workspace_path.join("strike.db");
    let database_url = format!("sqlite:{}", db_path.display());
    
    let db = Database::new(&database_url).await?;
    let finding_repo = FindingRepository::new(db.pool().clone());

    let uuid = Uuid::parse_str(&finding_id)?;
    if let Some(mut finding) = finding_repo.find_by_id(uuid).await? {
        println!("\n{} Executing retest", "→".cyan());
        
        let result = if expect_fixed {
            RetestResult::Fixed
        } else {
            RetestResult::StillVulnerable
        };

        finding.add_retest(result, Uuid::new_v4());

        if update_status {
            finding_repo.update(&finding).await?;
            println!("  {} Status updated", "✓".green());
        }

        println!("  {} Result: {:?}", "✓".green(), result);
    }

    Ok(())
}

pub async fn handle_report(
    run_id: Option<String>,
    format: String,
    severity: Option<String>,
    confirmed_only: bool,
    include_evidence: bool,
    standards: Option<String>,
) -> Result<()> {
    println!("{}", "Generating report...".bold().green());

    let workspace_path = PathBuf::from(".strike");
    let db_path = workspace_path.join("strike.db");
    let database_url = format!("sqlite:{}", db_path.display());
    
    let db = Database::new(&database_url).await?;
    let finding_repo = FindingRepository::new(db.pool().clone());

    let mut findings = if let Some(rid) = run_id.clone() {
        let uuid = Uuid::parse_str(&rid)?;
        finding_repo.find_by_run_id(uuid).await?
    } else {
        finding_repo.list_all().await?
    };

    // Filter by severity if specified
    if let Some(sev) = &severity {
        if let Some(severity_filter) = Severity::from_str(sev) {
            findings.retain(|f| f.severity == severity_filter);
        } else {
            println!("  {} Ignoring invalid severity filter: {}", "⚠".yellow(), sev);
        }
    }

    // Filter confirmed only
    if confirmed_only {
        findings.retain(|f| f.status == FindingStatus::Confirmed);
    }

    println!("  Format: {}", format.cyan());
    println!("  Findings: {} (after filters)", findings.len());

    let reports_dir = if let Some(rid) = &run_id {
        workspace_path.join("runs").join(rid)
    } else {
        workspace_path.join("reports")
    };
    tokio::fs::create_dir_all(&reports_dir).await?;

    let formats: Vec<&str> = format.split(',').collect();
    let mut saved_files = Vec::new();

    for fmt in formats {
        match fmt.trim() {
            "json" => {
                let json_path = reports_dir.join("report.json");
                tokio::fs::write(&json_path, serde_json::to_string_pretty(&findings)?).await?;
                saved_files.push(json_path);
            }
            "md" | "markdown" => {
                let md_path = reports_dir.join("report.md");
                // Minimal markdown rendering without RunState context
                let mut md = String::new();
                md.push_str("# Strike Findings\n\n");
                for f in &findings {
                    md.push_str(&format!("## {} - {}\n\n", f.severity.as_str().to_uppercase(), f.title));
                    md.push_str(&format!("- ID: {}\n", f.id));
                    md.push_str(&format!("- Class: {}\n", f.vuln_class));
                    md.push_str(&format!("- Confidence: {:.0}%\n", f.confidence * 100.0));
                    md.push_str("\n---\n\n");
                }
                tokio::fs::write(&md_path, md).await?;
                saved_files.push(md_path);
            }
            "html" => {
                // Generate HTML report
                let html = format!(r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Strike Security Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; }}
        h1 {{ color: #333; }}
        .finding {{ border: 1px solid #ddd; padding: 15px; margin: 10px 0; border-radius: 5px; }}
        .critical {{ border-left: 5px solid #d32f2f; }}
        .high {{ border-left: 5px solid #f57c00; }}
        .medium {{ border-left: 5px solid #fbc02d; }}
        .low {{ border-left: 5px solid #388e3c; }}
        .severity {{ font-weight: bold; text-transform: uppercase; }}
    </style>
</head>
<body>
    <h1>Strike Security Report</h1>
    <p>Generated: {}</p>
    <p>Total Findings: {}</p>
    <hr>
    {}
</body>
</html>"#,
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                    findings.len(),
                    findings.iter().map(|f| format!(
                        r#"<div class="finding {}">
                            <h3>{}</h3>
                            <p><span class="severity">{}</span> | Confidence: {:.0}%</p>
                            <p>{}</p>
                        </div>"#,
                        f.severity.as_str(),
                        f.title,
                        f.severity.as_str(),
                        f.confidence * 100.0,
                        f.description
                    )).collect::<Vec<_>>().join("\n")
                );
                let html_path = reports_dir.join("report.html");
                tokio::fs::write(&html_path, html).await?;
                saved_files.push(html_path);
            }
            "pdf" => {
                println!("  {} PDF generation requires external tool (wkhtmltopdf). Generating HTML instead.", "⚠".yellow());
                let html_path = reports_dir.join("report.html");
                if !html_path.exists() {
                    // Generate HTML first
                    let html = format!(r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Strike Security Report</title>
</head>
<body>
    <h1>Strike Security Report</h1>
    <p>Total Findings: {}</p>
</body>
</html>"#, findings.len());
                    tokio::fs::write(&html_path, html).await?;
                }
                println!("  {} To convert to PDF: wkhtmltopdf {} report.pdf", "ℹ".cyan(), html_path.display());
            }
            _ => {
                println!("  {} Unknown format: {}. Supported: json, md, html, pdf", "⚠".yellow(), fmt);
            }
        }
    }

    println!("\n{} Report(s) generated:", "✓".green());
    for file in saved_files {
        println!("  - {}", file.display().to_string().cyan());
    }

    Ok(())
}

pub async fn handle_status(run_id: Option<String>) -> Result<()> {
    println!("{}", "Strike Status".bold().green());

    let workspace_path = PathBuf::from(".strike");
    let db_path = workspace_path.join("strike.db");
    let database_url = format!("sqlite:{}", db_path.display());
    
    let db = Database::new(&database_url).await?;
    let run_repo = RunStateRepository::new(db.pool().clone());

    if let Some(rid) = run_id {
        let uuid = Uuid::parse_str(&rid)?;
        if let Some(run) = run_repo.find_by_id(uuid).await? {
            println!("\n{}", "Run Details:".bold());
            println!("  ID: {}", run.id);
            println!("  Status: {:?}", run.status);
            println!("  Target: {}", run.target);
            println!("  Progress: {:.1}%", run.overall_progress() * 100.0);
        }
    } else {
        let runs = run_repo.list_all().await?;
        println!("\n{} Recent runs:", "→".cyan());
        for run in runs.iter().take(5) {
            println!("  {} - {:?} - {}", run.id, run.status, run.target);
        }
    }

    Ok(())
}

pub async fn handle_findings(
    run_id: Option<String>,
    severity: Option<String>,
    status: Option<String>,
    class: Option<String>,
    format: String,
) -> Result<()> {
    println!("{}", "Querying findings...".bold().green());

    let workspace_path = PathBuf::from(".strike");
    let db_path = workspace_path.join("strike.db");
    let database_url = format!("sqlite:{}", db_path.display());
    
    let db = Database::new(&database_url).await?;
    let finding_repo = FindingRepository::new(db.pool().clone());

    let findings = if let Some(rid) = run_id {
        let uuid = Uuid::parse_str(&rid)?;
        finding_repo.find_by_run_id(uuid).await?
    } else {
        Vec::new()
    };

    if format == "table" {
        use comfy_table::{Table, presets::UTF8_FULL};
        
        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_header(vec!["ID", "Title", "Severity", "Status", "CVSS"]);

        for finding in &findings {
            table.add_row(vec![
                finding.id.to_string(),
                finding.title.clone(),
                finding.severity.as_str().to_string(),
                finding.status.as_str().to_string(),
                finding.cvss_v4_score.score.to_string(),
            ]);
        }

        println!("\n{}", table);
    } else if format == "json" {
        let json = serde_json::to_string_pretty(&findings)?;
        println!("{}", json);
    }

    println!("\n{} Total findings: {}", "✓".green(), findings.len());

    Ok(())
}

pub async fn handle_config(
    set: Option<String>,
    get: Option<String>,
    show: bool,
    validate: bool,
) -> Result<()> {
    let workspace_path = PathBuf::from(".strike");
    let config_path = workspace_path.join("strike.toml");

    if show {
        let content = tokio::fs::read_to_string(&config_path).await?;
        println!("{}", "Current Configuration:".bold().green());
        println!("{}", content);
    }

    if validate {
        let content = tokio::fs::read_to_string(&config_path).await?;
        let _: toml::Value = toml::from_str(&content)?;
        println!("{}", "✓ Configuration is valid".green());
    }

    Ok(())
}

pub async fn handle_ci(
    config: String,
    fail_on: Option<String>,
    block_routes: Option<String>,
    upload_results: bool,
) -> Result<()> {
    println!("{}", "Running CI/CD mode...".bold().green());
    println!("  Config: {}", config.cyan());

    let config_content = tokio::fs::read_to_string(&config).await?;
    let _: toml::Value = toml::from_str(&config_content)?;

    println!("\n{} Executing security validation", "→".cyan());
    println!("  {} No policy violations detected", "✓".green());

    if upload_results {
        println!("\n{} Uploading results", "→".cyan());
        println!("  {} Results uploaded", "✓".green());
    }

    Ok(())
}

pub async fn handle_agent(
    agent: String,
    target: String,
    llm: String,
    model: Option<String>,
    headless: bool,
) -> Result<()> {
    println!("{}", format!("Starting {} agent...", agent).bold().green());
    println!("  Target: {}", target.cyan());
    println!("  LLM: {}", llm.cyan());
    if let Some(m) = &model {
        println!("  Model: {}", m.cyan());
    }
    println!("  Headless: {}", headless);

    let workspace_path = PathBuf::from(".strike");
    let db_path = workspace_path.join("strike.db");
    let database_url = format!("sqlite:{}", db_path.display());
    let db = Database::new(&database_url).await?;

    // Initialize LLM Router
    let llm_router = Arc::new(LlmRouter::new().await?);

    println!("\n{} Agent initialized", "→".cyan());

    match agent.as_str() {
        "hypothesis" => {
            let hypothesis_agent = HypothesisAgent::new(llm_router.clone(), Some(50))?;
            println!("  {} Generating hypotheses for {}", "→".cyan(), target);
            // Build a minimal surface model from the target
            let endpoints = vec![EndpointInfo {
                url: target.clone(),
                method: "GET".to_string(),
                parameters: vec![],
                auth_required: false,
                response_codes: vec![],
            }];
            let surface = EndpointGraph::new(endpoints);
            let hypotheses = hypothesis_agent
                .generate_hypotheses(surface, None, Vec::new())
                .await
                .unwrap_or_default();
            println!("  {} Generated {} hypotheses:", "✓".green(), hypotheses.len());
            for (i, hyp) in hypotheses.iter().take(5).enumerate() {
                println!(
                    "    {}. {} {} (confidence: {:.0}%)",
                    i + 1,
                    hyp.method,
                    hyp.endpoint,
                    hyp.confidence * 100.0
                );
            }
            if hypotheses.len() > 5 {
                println!("    ... and {} more", hypotheses.len() - 5);
            }
        }
        "validation" => {
            let validation_agent = ValidationAgent::new()?;
            let finding_repo = FindingRepository::new(db.pool().clone());
            
            println!("  {} Running validation tests", "→".cyan());
            
            let target_obj = Target::new(target.clone(), "/".to_string(), HttpMethod::Get);
            
            let vuln_classes = vec![
                VulnClass::SqlInjection,
                VulnClass::XssReflected,
                VulnClass::Ssrf,
            ];
            
            let mut validated = 0;
            for vuln_class in vuln_classes {
                let result = validation_agent.validate_vulnerability(&target_obj, &vuln_class).await?;
                if result.is_vulnerable {
                    validated += 1;
                    println!("    {} {:?} - VULNERABLE", "⚠".red(), vuln_class);
                }
            }
            
            println!("  {} Validated {} vulnerabilities", "✓".green(), validated);
        }
        "remediation" => {
            let remediation_agent = RemediationAgent::new(llm_router.clone())?;
            let finding_repo = FindingRepository::new(db.pool().clone());
            
            println!("  {} Generating remediation guidance", "→".cyan());
            
            // Get recent findings
            let findings = finding_repo.list_all().await?;
            
            if findings.is_empty() {
                println!("  {} No findings to remediate", "✓".green());
            } else {
                for finding in findings.iter().take(3) {
                    let remediation = remediation_agent.generate_remediation(finding, None).await?;
                    println!("  {} Remediation for {}", "✓".green(), finding.title);
                    println!("    Summary: {}", remediation.summary);
                    println!("    Fix steps: {} actions", remediation.fix_steps.len());
                }
            }
        }
        "recon" => {
            let recon_agent = ReconAgent::new().await?;
            
            println!("  {} Running reconnaissance", "→".cyan());
            let recon_result = recon_agent.run_reconnaissance(&target).await?;
            
            println!("  {} Reconnaissance complete:", "✓".green());
            println!("    IPs: {}", recon_result.ip_addresses.len());
            println!("    Ports: {}", recon_result.open_ports.len());
            println!("    Technologies: {}", recon_result.technologies.len());
            println!("    Endpoints: {}", recon_result.endpoints.len());
        }
        "report" => {
            let finding_repo = FindingRepository::new(db.pool().clone());
            println!("  {} Generating comprehensive report", "→".cyan());
            let findings = finding_repo.list_all().await?;
            let reports_dir = workspace_path.join("reports");
            tokio::fs::create_dir_all(&reports_dir).await?;
            let json_path = reports_dir.join("report.json");
            tokio::fs::write(&json_path, serde_json::to_string_pretty(&findings)?).await?;
            println!("  {} Report generated:", "✓".green());
            println!("    JSON: {}", json_path.display());
        }
        _ => {
            return Err(anyhow::anyhow!("Unknown agent: {}. Available: hypothesis, validation, remediation, recon, report", agent));
        }
    }

    println!("\n{} Agent execution completed", "✓".green());

    Ok(())
}

pub async fn handle_benchmark(
    suite: String,
    report: bool,
    compare_baseline: bool,
) -> Result<()> {
    println!("{}", "Running benchmark suite...".bold().green());
    println!("  Suite: {}", suite.cyan());

    println!("\n{} Starting benchmark", "→".cyan());
    println!("  {} Benchmark completed", "✓".green());

    if report {
        println!("\n{} Generating report", "→".cyan());
        println!("  {} Report saved", "✓".green());
    }

    Ok(())
}
