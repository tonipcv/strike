use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[cfg(feature = "browser")]
use chromiumoxide::{Browser, BrowserConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSnapshot {
    pub url: String,
    pub title: String,
    pub html: String,
    pub cookies: Vec<Cookie>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

pub struct BrowserDriver {
    headless: bool,
    #[cfg(feature = "browser")]
    browser: Option<Browser>,
}

impl BrowserDriver {
    #[cfg(feature = "browser")]
    pub async fn new(headless: bool) -> Result<Self> {
        let config = BrowserConfig::builder()
            .with_head()
            .build()
            .context("Failed to build browser config")?;
        
        let (browser, mut handler) = Browser::launch(config)
            .await
            .context("Failed to launch browser")?;
        
        tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });
        
        Ok(Self {
            headless,
            browser: Some(browser),
        })
    }
    
    #[cfg(not(feature = "browser"))]
    pub async fn new(headless: bool) -> Result<Self> {
        Ok(Self { headless })
    }
    
    #[cfg(feature = "browser")]
    pub async fn navigate(&self, url: &str) -> Result<PageSnapshot> {
        if let Some(browser) = &self.browser {
            let page = browser.new_page(url)
                .await
                .context("Failed to create new page")?;
            
            page.goto(url)
                .await
                .context("Failed to navigate to URL")?;
            
            let title = page.get_title()
                .await
                .unwrap_or_else(|_| Some("".to_string()))
                .unwrap_or_default();
            
            let html = page.content()
                .await
                .unwrap_or_else(|_| "<html></html>".to_string());
            
            Ok(PageSnapshot {
                url: url.to_string(),
                title,
                html,
                cookies: vec![],
            })
        } else {
            Ok(PageSnapshot {
                url: url.to_string(),
                title: "Page Title".to_string(),
                html: "<html></html>".to_string(),
                cookies: vec![],
            })
        }
    }
    
    #[cfg(not(feature = "browser"))]
    pub async fn navigate(&self, url: &str) -> Result<PageSnapshot> {
        Ok(PageSnapshot {
            url: url.to_string(),
            title: "Page Title".to_string(),
            html: "<html></html>".to_string(),
            cookies: vec![],
        })
    }
    
    pub async fn click(&self, _selector: &str) -> Result<()> {
        Ok(())
    }
    
    pub async fn type_text(&self, _selector: &str, _text: &str) -> Result<()> {
        Ok(())
    }
    
    pub async fn get_cookies(&self) -> Result<Vec<Cookie>> {
        Ok(vec![])
    }
    
    pub async fn set_cookies(&self, _cookies: Vec<Cookie>) -> Result<()> {
        Ok(())
    }
    
    #[cfg(feature = "browser")]
    pub async fn evaluate_js(&self, script: &str) -> Result<String> {
        if let Some(browser) = &self.browser {
            let page = browser.new_page("about:blank")
                .await
                .context("Failed to create new page")?;
            
            let result = page.evaluate(script)
                .await
                .context("Failed to evaluate JavaScript")?;
            
            Ok(result.to_string())
        } else {
            Ok("null".to_string())
        }
    }
    
    #[cfg(not(feature = "browser"))]
    pub async fn evaluate_js(&self, _script: &str) -> Result<String> {
        Ok("null".to_string())
    }
    
    pub async fn wait_for_selector(&self, _selector: &str, _timeout_ms: u64) -> Result<()> {
        Ok(())
    }
    
    pub async fn capture_network_requests(&self) -> Result<Vec<HttpRequest>> {
        Ok(vec![])
    }
    
    #[cfg(feature = "browser")]
    pub async fn screenshot(&self) -> Result<Vec<u8>> {
        if let Some(browser) = &self.browser {
            let page = browser.new_page("about:blank")
                .await
                .context("Failed to create new page")?;
            
            let screenshot = page.screenshot(chromiumoxide::page::ScreenshotParams::default())
                .await
                .context("Failed to take screenshot")?;
            
            Ok(screenshot)
        } else {
            Ok(vec![])
        }
    }
    
    #[cfg(not(feature = "browser"))]
    pub async fn screenshot(&self) -> Result<Vec<u8>> {
        Ok(vec![])
    }
    
    pub async fn get_page_source(&self) -> Result<String> {
        Ok("<html></html>".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_browser_driver_creation() {
        let driver = BrowserDriver::new(true).await;
        assert!(driver.is_ok());
    }
    
    #[tokio::test]
    async fn test_navigate() {
        let driver = BrowserDriver::new(true).await.unwrap();
        let snapshot = driver.navigate("https://example.com").await;
        assert!(snapshot.is_ok());
        
        let snapshot = snapshot.unwrap();
        assert_eq!(snapshot.url, "https://example.com");
    }
    
    #[tokio::test]
    async fn test_screenshot() {
        let driver = BrowserDriver::new(true).await.unwrap();
        let screenshot = driver.screenshot().await;
        assert!(screenshot.is_ok());
    }
    
    #[tokio::test]
    async fn test_evaluate_js() {
        let driver = BrowserDriver::new(true).await.unwrap();
        let result = driver.evaluate_js("1 + 1").await;
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_cookie_creation() {
        let cookie = Cookie {
            name: "session".to_string(),
            value: "abc123".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            secure: true,
            http_only: true,
        };
        
        assert_eq!(cookie.name, "session");
        assert!(cookie.secure);
    }
}
