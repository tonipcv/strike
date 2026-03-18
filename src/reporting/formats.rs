
pub enum ReportFormat {
    Json,
    Markdown,
    Sarif,
    Html,
    Pdf,
}

impl ReportFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "md" | "markdown" => Some(Self::Markdown),
            "sarif" => Some(Self::Sarif),
            "html" => Some(Self::Html),
            "pdf" => Some(Self::Pdf),
            _ => None,
        }
    }
}
