use anyhow::Result;
use crate::models::VulnClass;

pub struct VulnDetector;

impl VulnDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detect_vulnerability_class(&self, pattern: &str) -> Option<VulnClass> {
        if pattern.contains("SQL") || pattern.contains("syntax error") {
            Some(VulnClass::SqlInjection)
        } else if pattern.contains("<script>") || pattern.contains("onerror") {
            Some(VulnClass::XssReflected)
        } else if pattern.contains("SSRF") || pattern.contains("localhost") {
            Some(VulnClass::Ssrf)
        } else {
            None
        }
    }
}
