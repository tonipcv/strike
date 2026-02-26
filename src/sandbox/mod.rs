use anyhow::Result;
use bollard::Docker;

pub struct Sandbox {
    docker: Docker,
}

impl Sandbox {
    pub async fn new() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;
        Ok(Self { docker })
    }

    pub async fn is_available(&self) -> bool {
        self.docker.ping().await.is_ok()
    }
}
