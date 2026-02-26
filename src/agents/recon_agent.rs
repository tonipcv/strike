use anyhow::Result;
use crate::tools::{HttpClient, PortScanner, DnsResolver};
use std::net::IpAddr;
use url::Url;

pub struct ReconAgent {
    http_client: HttpClient,
    port_scanner: PortScanner,
    dns_resolver: DnsResolver,
}

#[derive(Debug, Clone)]
pub struct ReconResult {
    pub target: String,
    pub ip_addresses: Vec<String>,
    pub open_ports: Vec<u16>,
    pub technologies: Vec<String>,
    pub endpoints: Vec<String>,
    pub subdomains: Vec<String>,
}

impl ReconAgent {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            http_client: HttpClient::new(50, 30)?,
            port_scanner: PortScanner::new(1000),
            dns_resolver: DnsResolver::new().await?,
        })
    }

    pub async fn run_reconnaissance(&self, target: &str) -> Result<ReconResult> {
        let url = Url::parse(target)?;
        let domain = url.host_str().ok_or_else(|| anyhow::anyhow!("Invalid target URL"))?;

        let ip_addresses = self.dns_resolver.resolve(domain).await?;

        let mut open_ports = Vec::new();
        if let Some(first_ip) = ip_addresses.first() {
            if let Ok(ip) = first_ip.parse::<IpAddr>() {
                open_ports = self.port_scanner.scan_common_ports(ip).await?;
            }
        }

        let technologies = self.detect_technologies(target).await?;
        let endpoints = self.discover_endpoints(target).await?;

        Ok(ReconResult {
            target: target.to_string(),
            ip_addresses,
            open_ports,
            technologies,
            endpoints,
            subdomains: Vec::new(),
        })
    }

    async fn detect_technologies(&self, target: &str) -> Result<Vec<String>> {
        let mut technologies = Vec::new();

        match self.http_client.get(target).await {
            Ok(response) => {
                if let Some(server) = response.headers().get("server") {
                    if let Ok(server_str) = server.to_str() {
                        technologies.push(server_str.to_string());
                    }
                }

                if let Some(powered_by) = response.headers().get("x-powered-by") {
                    if let Ok(powered_str) = powered_by.to_str() {
                        technologies.push(powered_str.to_string());
                    }
                }
            }
            Err(_) => {}
        }

        Ok(technologies)
    }

    async fn discover_endpoints(&self, target: &str) -> Result<Vec<String>> {
        let mut endpoints = Vec::new();
        
        let common_paths = vec![
            "/api", "/admin", "/login", "/dashboard", "/users", "/api/v1",
            "/api/v2", "/graphql", "/swagger", "/docs",
        ];

        for path in common_paths {
            let url = format!("{}{}", target.trim_end_matches('/'), path);
            endpoints.push(url);
        }

        Ok(endpoints)
    }

    pub async fn enumerate_subdomains(&self, domain: &str) -> Result<Vec<String>> {
        let common_subdomains = vec![
            "www", "api", "admin", "dev", "staging", "test", "mail", "ftp",
        ];

        let mut found_subdomains = Vec::new();

        for subdomain in common_subdomains {
            let full_domain = format!("{}.{}", subdomain, domain);
            if let Ok(_) = self.dns_resolver.resolve(&full_domain).await {
                found_subdomains.push(full_domain);
            }
        }

        Ok(found_subdomains)
    }
}
