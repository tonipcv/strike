use anyhow::Result;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;

pub struct PortScanner {
    timeout_ms: u64,
}

impl PortScanner {
    pub fn new(timeout_ms: u64) -> Self {
        Self { timeout_ms }
    }

    pub async fn scan_port(&self, ip: IpAddr, port: u16) -> bool {
        let socket_addr = SocketAddr::new(ip, port);
        
        match timeout(
            Duration::from_millis(self.timeout_ms),
            TcpStream::connect(socket_addr),
        )
        .await
        {
            Ok(Ok(_)) => true,
            _ => false,
        }
    }

    pub async fn scan_ports(&self, ip: IpAddr, ports: Vec<u16>) -> Result<Vec<u16>> {
        let mut open_ports = Vec::new();

        for port in ports {
            if self.scan_port(ip, port).await {
                open_ports.push(port);
            }
        }

        Ok(open_ports)
    }

    pub async fn scan_common_ports(&self, ip: IpAddr) -> Result<Vec<u16>> {
        let common_ports = vec![
            21, 22, 23, 25, 53, 80, 110, 111, 135, 139, 143, 443, 445, 993, 995, 1723, 3306,
            3389, 5900, 8080, 8443,
        ];

        self.scan_ports(ip, common_ports).await
    }
}
