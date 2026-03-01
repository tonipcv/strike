use anyhow::Result;
use trust_dns_resolver::TokioAsyncResolver;
use trust_dns_resolver::config::*;

pub struct DnsResolver {
    resolver: TokioAsyncResolver,
}

impl DnsResolver {
    pub fn new() -> Self {
        let resolver = TokioAsyncResolver::tokio(
            ResolverConfig::default(),
            ResolverOpts::default(),
        );

        Self { resolver }
    }

    pub async fn resolve(&self, domain: &str) -> Result<Vec<String>> {
        let response = self.resolver.lookup_ip(domain).await?;
        let ips: Vec<String> = response.iter().map(|ip| ip.to_string()).collect();
        Ok(ips)
    }
}
