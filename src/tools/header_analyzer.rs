use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderReport {
    pub score: f32,
    pub csp_analysis: Option<CspAnalysis>,
    pub hsts_status: HstsStatus,
    pub cors_analysis: Option<CorsAnalysis>,
    pub has_x_frame_options: bool,
    pub has_content_type_options: bool,
    pub missing_headers: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CspAnalysis {
    pub present: bool,
    pub directives: Vec<String>,
    pub unsafe_inline: bool,
    pub unsafe_eval: bool,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HstsStatus {
    Present { max_age: u64, include_subdomains: bool },
    Missing,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsAnalysis {
    pub allow_origin: Option<String>,
    pub allow_credentials: bool,
    pub wildcard_origin: bool,
    pub security_risk: bool,
}

pub struct HeaderAnalyzer;

impl HeaderAnalyzer {
    pub fn new() -> Self {
        Self
    }
    
    pub fn analyze(&self, headers: &HashMap<String, String>) -> HeaderReport {
        let csp_analysis = self.check_csp(headers);
        let hsts_status = self.check_hsts(headers);
        let cors_analysis = self.check_cors(headers, None);
        let has_x_frame_options = self.check_x_frame_options(headers);
        let has_content_type_options = self.check_content_type_sniffing(headers);
        
        let mut missing_headers = Vec::new();
        let mut recommendations = Vec::new();
        
        if csp_analysis.is_none() {
            missing_headers.push("Content-Security-Policy".to_string());
            recommendations.push("Add Content-Security-Policy header".to_string());
        }
        
        if matches!(hsts_status, HstsStatus::Missing) {
            missing_headers.push("Strict-Transport-Security".to_string());
            recommendations.push("Add HSTS header with max-age >= 31536000".to_string());
        }
        
        if !has_x_frame_options {
            missing_headers.push("X-Frame-Options".to_string());
            recommendations.push("Add X-Frame-Options: DENY or SAMEORIGIN".to_string());
        }
        
        if !has_content_type_options {
            missing_headers.push("X-Content-Type-Options".to_string());
            recommendations.push("Add X-Content-Type-Options: nosniff".to_string());
        }
        
        let score = self.calculate_score(&csp_analysis, &hsts_status, has_x_frame_options, has_content_type_options);
        
        HeaderReport {
            score,
            csp_analysis,
            hsts_status,
            cors_analysis,
            has_x_frame_options,
            has_content_type_options,
            missing_headers,
            recommendations,
        }
    }
    
    pub fn check_csp(&self, headers: &HashMap<String, String>) -> Option<CspAnalysis> {
        let csp = headers.get("content-security-policy")
            .or_else(|| headers.get("Content-Security-Policy"))?;
        
        let directives: Vec<String> = csp.split(';')
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty())
            .collect();
        
        let unsafe_inline = csp.contains("'unsafe-inline'");
        let unsafe_eval = csp.contains("'unsafe-eval'");
        
        let mut score: f32 = 100.0;
        if unsafe_inline { score -= 30.0; }
        if unsafe_eval { score -= 20.0; }
        if directives.len() < 3 { score -= 20.0; }
        
        Some(CspAnalysis {
            present: true,
            directives,
            unsafe_inline,
            unsafe_eval,
            score: score.max(0.0),
        })
    }
    
    pub fn check_hsts(&self, headers: &HashMap<String, String>) -> HstsStatus {
        let hsts = headers.get("strict-transport-security")
            .or_else(|| headers.get("Strict-Transport-Security"));
        
        match hsts {
            Some(value) => {
                let max_age = value.split(';')
                    .find(|part| part.trim().starts_with("max-age="))
                    .and_then(|part| part.trim().strip_prefix("max-age="))
                    .and_then(|age| age.parse::<u64>().ok());
                
                let include_subdomains = value.contains("includeSubDomains");
                
                match max_age {
                    Some(age) => HstsStatus::Present {
                        max_age: age,
                        include_subdomains,
                    },
                    None => HstsStatus::Invalid,
                }
            }
            None => HstsStatus::Missing,
        }
    }
    
    pub fn check_cors(&self, headers: &HashMap<String, String>, _origin: Option<&str>) -> Option<CorsAnalysis> {
        let allow_origin = headers.get("access-control-allow-origin")
            .or_else(|| headers.get("Access-Control-Allow-Origin"))
            .map(|s| s.to_string());
        
        let allow_credentials = headers.get("access-control-allow-credentials")
            .or_else(|| headers.get("Access-Control-Allow-Credentials"))
            .map(|s| s == "true")
            .unwrap_or(false);
        
        let wildcard_origin = allow_origin.as_ref().map(|o| o == "*").unwrap_or(false);
        let security_risk = wildcard_origin && allow_credentials;
        
        Some(CorsAnalysis {
            allow_origin,
            allow_credentials,
            wildcard_origin,
            security_risk,
        })
    }
    
    pub fn check_x_frame_options(&self, headers: &HashMap<String, String>) -> bool {
        headers.contains_key("x-frame-options") || headers.contains_key("X-Frame-Options")
    }
    
    pub fn check_content_type_sniffing(&self, headers: &HashMap<String, String>) -> bool {
        headers.get("x-content-type-options")
            .or_else(|| headers.get("X-Content-Type-Options"))
            .map(|v| v.to_lowercase() == "nosniff")
            .unwrap_or(false)
    }
    
    pub fn score(&self, headers: &HashMap<String, String>) -> f32 {
        let report = self.analyze(headers);
        report.score
    }
    
    fn calculate_score(&self, csp: &Option<CspAnalysis>, hsts: &HstsStatus, x_frame: bool, content_type: bool) -> f32 {
        let mut score = 0.0;
        
        if let Some(csp_analysis) = csp {
            score += csp_analysis.score * 0.4;
        }
        
        if matches!(hsts, HstsStatus::Present { .. }) {
            score += 25.0;
        }
        
        if x_frame {
            score += 15.0;
        }
        
        if content_type {
            score += 10.0;
        }
        
        score.min(100.0)
    }
}

impl Default for HeaderAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_analyzer_creation() {
        let analyzer = HeaderAnalyzer::new();
        assert_eq!(analyzer, HeaderAnalyzer);
    }
    
    #[test]
    fn test_check_hsts_present() {
        let analyzer = HeaderAnalyzer::new();
        let mut headers = HashMap::new();
        headers.insert("Strict-Transport-Security".to_string(), "max-age=31536000; includeSubDomains".to_string());
        
        let status = analyzer.check_hsts(&headers);
        assert!(matches!(status, HstsStatus::Present { max_age: 31536000, include_subdomains: true }));
    }
    
    #[test]
    fn test_check_hsts_missing() {
        let analyzer = HeaderAnalyzer::new();
        let headers = HashMap::new();
        
        let status = analyzer.check_hsts(&headers);
        assert_eq!(status, HstsStatus::Missing);
    }
    
    #[test]
    fn test_check_csp() {
        let analyzer = HeaderAnalyzer::new();
        let mut headers = HashMap::new();
        headers.insert("Content-Security-Policy".to_string(), "default-src 'self'; script-src 'self'".to_string());
        
        let csp = analyzer.check_csp(&headers);
        assert!(csp.is_some());
        
        let csp = csp.unwrap();
        assert!(csp.present);
        assert!(!csp.unsafe_inline);
    }
    
    #[test]
    fn test_check_cors_wildcard() {
        let analyzer = HeaderAnalyzer::new();
        let mut headers = HashMap::new();
        headers.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
        headers.insert("Access-Control-Allow-Credentials".to_string(), "true".to_string());
        
        let cors = analyzer.check_cors(&headers, None);
        assert!(cors.is_some());
        
        let cors = cors.unwrap();
        assert!(cors.wildcard_origin);
        assert!(cors.security_risk);
    }
    
    #[test]
    fn test_analyze_headers() {
        let analyzer = HeaderAnalyzer::new();
        let mut headers = HashMap::new();
        headers.insert("Strict-Transport-Security".to_string(), "max-age=31536000".to_string());
        headers.insert("X-Frame-Options".to_string(), "DENY".to_string());
        headers.insert("X-Content-Type-Options".to_string(), "nosniff".to_string());
        
        let report = analyzer.analyze(&headers);
        assert!(report.score > 50.0);
        assert!(report.has_x_frame_options);
        assert!(report.has_content_type_options);
    }
}
