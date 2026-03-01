use anyhow::Result;
use crate::tools::HttpClient;
use url::Url;

pub struct ReconAgent {
    http_client: HttpClient,
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
        })
    }

    pub async fn run_reconnaissance(&self, target: &str) -> Result<ReconResult> {
        let url = Url::parse(target)?;
        let domain = url.host_str().ok_or_else(|| anyhow::anyhow!("Invalid target URL"))?;

        let ip_addresses: Vec<String> = vec![];
        
        let mut open_ports = Vec::new();

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
            "blog", "shop", "store", "portal", "app", "mobile", "m", "beta",
            "alpha", "demo", "sandbox", "vpn", "cdn", "static", "assets",
            "media", "images", "files", "docs", "help", "support", "status",
        ];

        let mut found_subdomains = Vec::new();

        for subdomain in common_subdomains {
            let full_domain = format!("{}.{}", subdomain, domain);
            
            // Try HTTP/HTTPS connection as DNS alternative
            let test_urls = vec![
                format!("https://{}", full_domain),
                format!("http://{}", full_domain),
            ];
            
            for url in test_urls {
                match self.http_client.get(&url).await {
                    Ok(_) => {
                        found_subdomains.push(full_domain.clone());
                        break;
                    }
                    Err(_) => continue,
                }
            }
        }

        Ok(found_subdomains)
    }
}
