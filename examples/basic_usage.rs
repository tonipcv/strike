use strike_security::models::*;
use strike_security::storage::*;
use strike_security::agents::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Strike Security Platform - Basic Usage Example");
    println!("==============================================\n");

    let database_url = "sqlite::memory:";
    let db = Database::new(database_url).await?;
    db.initialize().await?;

    let run_state = RunState::new(
        "https://example.com".to_string(),
        RunProfile::Web,
        EnvironmentTag::Local,
        RunConfig::default(),
    );

    println!("✓ Created run state: {}", run_state.id);
    println!("  Target: {}", run_state.target);
    println!("  Profile: {:?}", run_state.profile);
    println!("  Environment: {:?}", run_state.environment);

    let roe = RulesOfEngagement::default();
    let scope_agent = ScopeAgent::new(roe);

    println!("\n✓ Initialized ScopeAgent");
    
    match scope_agent.validate_target("https://example.com") {
        Ok(_) => println!("  ✓ Target validation passed"),
        Err(e) => println!("  ✗ Target validation failed: {}", e),
    }

    let target = Target::new(
        "https://example.com".to_string(),
        "/api/users/1".to_string(),
        HttpMethod::Get,
    );

    println!("\n✓ Created target:");
    println!("  URL: {}", target.full_url());
    println!("  Method: {}", target.method.as_str());

    let cvss = CvssV4Score::calculate(BaseMetrics {
        attack_vector: AttackVector::Network,
        attack_complexity: AttackComplexity::Low,
        attack_requirements: AttackRequirements::None,
        privileges_required: PrivilegesRequired::None,
        user_interaction: UserInteraction::None,
        confidentiality: Impact::High,
        integrity: Impact::Low,
        availability: Impact::None,
    });

    println!("\n✓ CVSS v4.0 Score calculated:");
    println!("  Score: {}", cvss.score);
    println!("  Severity: {}", cvss.severity);
    println!("  Vector: {}", cvss.vector);

    let vuln_class = VulnClass::Idor;
    println!("\n✓ Vulnerability Class: {}", vuln_class);
    
    if let Some(owasp) = vuln_class.owasp_top10_mapping() {
        println!("  OWASP Top 10: {}", owasp);
    }
    
    if let Some(api) = vuln_class.owasp_api_top10_mapping() {
        println!("  OWASP API Top 10: {}", api);
    }
    
    if let Some(cwe) = vuln_class.cwe_id() {
        println!("  CWE: CWE-{}", cwe);
    }

    println!("\n✓ Strike Security Platform initialized successfully!");
    println!("  All components working as expected.");

    Ok(())
}
