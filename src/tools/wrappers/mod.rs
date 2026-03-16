pub mod nmap;
pub mod nuclei;
pub mod nikto;
pub mod sqlmap;
pub mod subfinder;
pub mod httpx;
pub mod gobuster;
pub mod ffuf;

pub use nmap::NmapWrapper;
pub use nuclei::NucleiWrapper;
pub use nikto::NiktoWrapper;
pub use sqlmap::SQLMapWrapper;
pub use subfinder::SubfinderWrapper;
pub use httpx::HttpxWrapper;
pub use gobuster::GobusterWrapper;
pub use ffuf::FfufWrapper;

use crate::tools::r#trait::Tool;
use std::sync::Arc;

/// Get all available tool wrappers
pub fn get_all_wrappers() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(NmapWrapper::new()),
        Arc::new(NucleiWrapper::new()),
        Arc::new(NiktoWrapper::new()),
        Arc::new(SQLMapWrapper::new()),
        Arc::new(SubfinderWrapper::new()),
        Arc::new(HttpxWrapper::new()),
        Arc::new(GobusterWrapper::new()),
        Arc::new(FfufWrapper::new()),
    ]
}

/// Get tool wrapper by name
pub fn get_wrapper_by_name(name: &str) -> Option<Arc<dyn Tool>> {
    match name.to_lowercase().as_str() {
        "nmap" => Some(Arc::new(NmapWrapper::new())),
        "nuclei" => Some(Arc::new(NucleiWrapper::new())),
        "nikto" => Some(Arc::new(NiktoWrapper::new())),
        "sqlmap" => Some(Arc::new(SQLMapWrapper::new())),
        "subfinder" => Some(Arc::new(SubfinderWrapper::new())),
        "httpx" => Some(Arc::new(HttpxWrapper::new())),
        "gobuster" => Some(Arc::new(GobusterWrapper::new())),
        "ffuf" => Some(Arc::new(FfufWrapper::new())),
        _ => None,
    }
}

/// Check which tools are installed
pub async fn check_installed_tools() -> Vec<String> {
    let mut installed = Vec::new();
    
    for tool in get_all_wrappers() {
        if tool.check_installation().await.unwrap_or(false) {
            installed.push(tool.name().to_string());
        }
    }
    
    installed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_wrappers() {
        let wrappers = get_all_wrappers();
        assert_eq!(wrappers.len(), 4);
    }

    #[test]
    fn test_get_wrapper_by_name() {
        assert!(get_wrapper_by_name("nmap").is_some());
        assert!(get_wrapper_by_name("nuclei").is_some());
        assert!(get_wrapper_by_name("nikto").is_some());
        assert!(get_wrapper_by_name("sqlmap").is_some());
        assert!(get_wrapper_by_name("nonexistent").is_none());
    }

    #[tokio::test]
    async fn test_check_installed_tools() {
        let installed = check_installed_tools().await;
        // At least some tools should be available in CI/CD
        println!("Installed tools: {:?}", installed);
    }
}
