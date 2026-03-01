use anyhow::Result;
use bollard::Docker;
use bollard::container::{Config, CreateContainerOptions, StartContainerOptions, RemoveContainerOptions};
use bollard::image::CreateImageOptions;
use futures::StreamExt;
use std::default::Default;

pub struct Sandbox {
    docker: Option<Docker>,
}

impl Sandbox {
    pub async fn new() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults().ok();
        
        Ok(Self { docker })
    }
    
    pub fn is_available(&self) -> bool {
        self.docker.is_some()
    }
    
    pub async fn execute_in_sandbox(&self, command: &str) -> Result<String> {
        if !self.is_available() {
            anyhow::bail!("Docker not available");
        }
        
        let docker = self.docker.as_ref().unwrap();
        
        // Pull alpine image if not exists
        let image = "alpine:latest";
        let mut stream = docker.create_image(
            Some(CreateImageOptions {
                from_image: image,
                ..Default::default()
            }),
            None,
            None,
        );
        
        while let Some(_) = stream.next().await {}
        
        // Create container
        let container_name = format!("strike-sandbox-{}", uuid::Uuid::new_v4());
        let config = Config {
            image: Some(image),
            cmd: Some(vec!["sh", "-c", command]),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };
        
        let container = docker
            .create_container(
                Some(CreateContainerOptions {
                    name: container_name.clone(),
                    ..Default::default()
                }),
                config,
            )
            .await?;
        
        // Start container
        docker
            .start_container(&container.id, None::<StartContainerOptions<String>>)
            .await?;
        
        // Wait for container to finish
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Get logs
        let mut output = String::new();
        let logs_stream = docker.logs::<String>(
            &container.id,
            Some(bollard::container::LogsOptions {
                stdout: true,
                stderr: true,
                ..Default::default()
            }),
        );
        
        let logs: Vec<_> = logs_stream.collect().await;
        for log in logs {
            if let Ok(log_output) = log {
                output.push_str(&log_output.to_string());
            }
        }
        
        // Remove container
        docker
            .remove_container(
                &container.id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await?;
        
        Ok(output)
    }
    
    pub async fn test_payload_isolated(&self, payload: &str, target: &str) -> Result<SandboxResult> {
        if !self.is_available() {
            return Ok(SandboxResult {
                executed: false,
                output: String::new(),
                safe: false,
                error: Some("Docker not available".to_string()),
            });
        }
        
        // Execute payload in isolated container
        let command = format!("curl -X POST {} -d '{}'", target, payload);
        
        match self.execute_in_sandbox(&command).await {
            Ok(output) => Ok(SandboxResult {
                executed: true,
                output: output.clone(),
                safe: !output.contains("error") && !output.contains("exception"),
                error: None,
            }),
            Err(e) => Ok(SandboxResult {
                executed: false,
                output: String::new(),
                safe: false,
                error: Some(e.to_string()),
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SandboxResult {
    pub executed: bool,
    pub output: String,
    pub safe: bool,
    pub error: Option<String>,
}
