use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VulnClass {
    Idor,
    Bola,
    Bfla,
    PrivilegeEscalation,
    PathTraversal,
    MassAssignment,
    SqlInjection,
    NoSqlInjection,
    LdapInjection,
    OsCommandInjection,
    Ssti,
    XPathInjection,
    BrokenAuthentication,
    SessionFixation,
    TokenForgery,
    JwtWeakness,
    OAuth2Misconfiguration,
    TwoFactorBypass,
    XssReflected,
    XssStored,
    XssDom,
    Csrf,
    Clickjacking,
    OpenRedirect,
    Ssrf,
    Xxe,
    Deserialization,
    FileUploadAbuse,
    RaceCondition,
    MassDataExposure,
    UnrestrictedResourceConsumption,
    SecurityMisconfiguration,
    ImproperAssetManagement,
    WeakTls,
    InsecureHeaders,
    DefaultCredentials,
    SensitiveDataExposure,
    VerboseErrorMessages,
}

impl VulnClass {
    pub fn category(&self) -> VulnCategory {
        match self {
            Self::Idor | Self::Bola | Self::Bfla | Self::PrivilegeEscalation 
            | Self::PathTraversal | Self::MassAssignment => VulnCategory::AccessControl,
            
            Self::SqlInjection | Self::NoSqlInjection | Self::LdapInjection 
            | Self::OsCommandInjection | Self::Ssti | Self::XPathInjection => VulnCategory::Injection,
            
            Self::BrokenAuthentication | Self::SessionFixation | Self::TokenForgery 
            | Self::JwtWeakness | Self::OAuth2Misconfiguration | Self::TwoFactorBypass => VulnCategory::AuthAndSession,
            
            Self::XssReflected | Self::XssStored | Self::XssDom | Self::Csrf 
            | Self::Clickjacking | Self::OpenRedirect => VulnCategory::ClientSide,
            
            Self::Ssrf | Self::Xxe | Self::Deserialization | Self::FileUploadAbuse 
            | Self::RaceCondition => VulnCategory::ServerSide,
            
            Self::MassDataExposure | Self::UnrestrictedResourceConsumption 
            | Self::SecurityMisconfiguration | Self::ImproperAssetManagement => VulnCategory::ApiSpecific,
            
            Self::WeakTls | Self::InsecureHeaders | Self::DefaultCredentials 
            | Self::SensitiveDataExposure | Self::VerboseErrorMessages => VulnCategory::CryptoAndConfig,
        }
    }

    pub fn owasp_top10_mapping(&self) -> Option<&'static str> {
        match self {
            Self::BrokenAuthentication | Self::SessionFixation | Self::TokenForgery 
            | Self::JwtWeakness | Self::OAuth2Misconfiguration | Self::TwoFactorBypass => {
                Some("A07:2021 - Identification and Authentication Failures")
            },
            Self::SqlInjection | Self::NoSqlInjection | Self::LdapInjection 
            | Self::OsCommandInjection | Self::Ssti | Self::XPathInjection => {
                Some("A03:2021 - Injection")
            },
            Self::Idor | Self::Bola | Self::Bfla | Self::PrivilegeEscalation | Self::PathTraversal => {
                Some("A01:2021 - Broken Access Control")
            },
            Self::XssReflected | Self::XssStored | Self::XssDom => {
                Some("A03:2021 - Injection")
            },
            Self::SensitiveDataExposure | Self::WeakTls => {
                Some("A02:2021 - Cryptographic Failures")
            },
            Self::SecurityMisconfiguration | Self::InsecureHeaders | Self::VerboseErrorMessages => {
                Some("A05:2021 - Security Misconfiguration")
            },
            Self::Ssrf => Some("A10:2021 - Server-Side Request Forgery"),
            Self::Deserialization => Some("A08:2021 - Software and Data Integrity Failures"),
            _ => None,
        }
    }

    pub fn owasp_api_top10_mapping(&self) -> Option<&'static str> {
        match self {
            Self::Bola => Some("API1:2023 - Broken Object Level Authorization"),
            Self::BrokenAuthentication | Self::JwtWeakness | Self::OAuth2Misconfiguration => {
                Some("API2:2023 - Broken Authentication")
            },
            Self::Bfla => Some("API5:2023 - Broken Function Level Authorization"),
            Self::MassDataExposure => Some("API3:2023 - Broken Object Property Level Authorization"),
            Self::UnrestrictedResourceConsumption => Some("API4:2023 - Unrestricted Resource Consumption"),
            Self::SecurityMisconfiguration | Self::DefaultCredentials => {
                Some("API8:2023 - Security Misconfiguration")
            },
            Self::ImproperAssetManagement => Some("API9:2023 - Improper Inventory Management"),
            Self::Ssrf => Some("API7:2023 - Server Side Request Forgery"),
            _ => None,
        }
    }

    pub fn cwe_id(&self) -> Option<u32> {
        match self {
            Self::Idor | Self::Bola => Some(639),
            Self::SqlInjection => Some(89),
            Self::XssReflected | Self::XssStored | Self::XssDom => Some(79),
            Self::Csrf => Some(352),
            Self::Ssrf => Some(918),
            Self::PathTraversal => Some(22),
            Self::OsCommandInjection => Some(78),
            Self::Xxe => Some(611),
            Self::Deserialization => Some(502),
            Self::BrokenAuthentication => Some(287),
            Self::SessionFixation => Some(384),
            Self::WeakTls => Some(326),
            Self::SensitiveDataExposure => Some(200),
            _ => None,
        }
    }
}

impl fmt::Display for VulnClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Idor => "IDOR",
            Self::Bola => "BOLA",
            Self::Bfla => "BFLA",
            Self::PrivilegeEscalation => "Privilege Escalation",
            Self::PathTraversal => "Path Traversal",
            Self::MassAssignment => "Mass Assignment",
            Self::SqlInjection => "SQL Injection",
            Self::NoSqlInjection => "NoSQL Injection",
            Self::LdapInjection => "LDAP Injection",
            Self::OsCommandInjection => "OS Command Injection",
            Self::Ssti => "SSTI",
            Self::XPathInjection => "XPath Injection",
            Self::BrokenAuthentication => "Broken Authentication",
            Self::SessionFixation => "Session Fixation",
            Self::TokenForgery => "Token Forgery",
            Self::JwtWeakness => "JWT Weakness",
            Self::OAuth2Misconfiguration => "OAuth2 Misconfiguration",
            Self::TwoFactorBypass => "2FA Bypass",
            Self::XssReflected => "XSS (Reflected)",
            Self::XssStored => "XSS (Stored)",
            Self::XssDom => "XSS (DOM)",
            Self::Csrf => "CSRF",
            Self::Clickjacking => "Clickjacking",
            Self::OpenRedirect => "Open Redirect",
            Self::Ssrf => "SSRF",
            Self::Xxe => "XXE",
            Self::Deserialization => "Deserialization",
            Self::FileUploadAbuse => "File Upload Abuse",
            Self::RaceCondition => "Race Condition",
            Self::MassDataExposure => "Mass Data Exposure",
            Self::UnrestrictedResourceConsumption => "Unrestricted Resource Consumption",
            Self::SecurityMisconfiguration => "Security Misconfiguration",
            Self::ImproperAssetManagement => "Improper Asset Management",
            Self::WeakTls => "Weak TLS",
            Self::InsecureHeaders => "Insecure Headers",
            Self::DefaultCredentials => "Default Credentials",
            Self::SensitiveDataExposure => "Sensitive Data Exposure",
            Self::VerboseErrorMessages => "Verbose Error Messages",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VulnCategory {
    AccessControl,
    Injection,
    AuthAndSession,
    ClientSide,
    ServerSide,
    ApiSpecific,
    CryptoAndConfig,
}

impl fmt::Display for VulnCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::AccessControl => "Access Control",
            Self::Injection => "Injection",
            Self::AuthAndSession => "Authentication & Session",
            Self::ClientSide => "Client-Side",
            Self::ServerSide => "Server-Side",
            Self::ApiSpecific => "API-Specific",
            Self::CryptoAndConfig => "Crypto & Configuration",
        };
        write!(f, "{}", s)
    }
}
