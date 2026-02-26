use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub url: String,
    pub endpoint: String,
    pub method: HttpMethod,
    pub parameter: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Patch => "PATCH",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(Self::Get),
            "POST" => Some(Self::Post),
            "PUT" => Some(Self::Put),
            "DELETE" => Some(Self::Delete),
            "PATCH" => Some(Self::Patch),
            "HEAD" => Some(Self::Head),
            "OPTIONS" => Some(Self::Options),
            _ => None,
        }
    }
}

impl Target {
    pub fn new(url: String, endpoint: String, method: HttpMethod) -> Self {
        Self {
            url,
            endpoint,
            method,
            parameter: None,
        }
    }

    pub fn with_parameter(mut self, parameter: String) -> Self {
        self.parameter = Some(parameter);
        self
    }

    pub fn full_url(&self) -> String {
        if self.endpoint.starts_with("http") {
            self.endpoint.clone()
        } else {
            format!("{}{}", self.url.trim_end_matches('/'), self.endpoint)
        }
    }

    pub fn is_valid(&self) -> bool {
        Url::parse(&self.full_url()).is_ok()
    }
}
