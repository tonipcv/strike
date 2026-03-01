use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio::sync::mpsc;

pub type StreamChunk = String;
pub type LlmStream = Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>;

#[derive(Debug, Clone)]
pub struct StreamConfig {
    pub buffer_size: usize,
    pub chunk_timeout_ms: u64,
    pub enable_backpressure: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            buffer_size: 100,
            chunk_timeout_ms: 5000,
            enable_backpressure: true,
        }
    }
}

#[async_trait]
pub trait StreamingLlmProvider: Send + Sync {
    async fn stream_completion(&self, prompt: &str) -> Result<LlmStream>;
    
    fn supports_streaming(&self) -> bool {
        true
    }
}

pub struct StreamCollector {
    chunks: Vec<String>,
    total_bytes: usize,
}

impl StreamCollector {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            total_bytes: 0,
        }
    }
    
    pub fn add_chunk(&mut self, chunk: String) {
        self.total_bytes += chunk.len();
        self.chunks.push(chunk);
    }
    
    pub fn get_full_text(&self) -> String {
        self.chunks.join("")
    }
    
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }
    
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }
    
    pub fn clear(&mut self) {
        self.chunks.clear();
        self.total_bytes = 0;
    }
}

impl Default for StreamCollector {
    fn default() -> Self {
        Self::new()
    }
}

pub struct StreamProcessor {
    config: StreamConfig,
}

impl StreamProcessor {
    pub fn new(config: StreamConfig) -> Self {
        Self { config }
    }
    
    pub async fn process_stream<F>(&self, mut stream: LlmStream, mut callback: F) -> Result<String>
    where
        F: FnMut(&str) + Send,
    {
        use futures::StreamExt;
        
        let mut collector = StreamCollector::new();
        
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            callback(&chunk);
            collector.add_chunk(chunk);
        }
        
        Ok(collector.get_full_text())
    }
    
    pub async fn collect_stream(&self, mut stream: LlmStream) -> Result<String> {
        use futures::StreamExt;
        
        let mut collector = StreamCollector::new();
        
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            collector.add_chunk(chunk);
        }
        
        Ok(collector.get_full_text())
    }
    
    pub fn create_channel_stream(&self) -> (mpsc::Sender<Result<String>>, LlmStream) {
        let (tx, mut rx) = mpsc::channel(self.config.buffer_size);
        
        let stream = Box::pin(async_stream::stream! {
            while let Some(item) = rx.recv().await {
                yield item;
            }
        });
        
        (tx, stream)
    }
}

pub struct StreamMetrics {
    pub chunks_received: usize,
    pub total_bytes: usize,
    pub duration_ms: u64,
    pub avg_chunk_size: f64,
    pub throughput_bytes_per_sec: f64,
}

impl StreamMetrics {
    pub fn new() -> Self {
        Self {
            chunks_received: 0,
            total_bytes: 0,
            duration_ms: 0,
            avg_chunk_size: 0.0,
            throughput_bytes_per_sec: 0.0,
        }
    }
    
    pub fn calculate(&mut self) {
        if self.chunks_received > 0 {
            self.avg_chunk_size = self.total_bytes as f64 / self.chunks_received as f64;
        }
        
        if self.duration_ms > 0 {
            let duration_sec = self.duration_ms as f64 / 1000.0;
            self.throughput_bytes_per_sec = self.total_bytes as f64 / duration_sec;
        }
    }
}

impl Default for StreamMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_config_default() {
        let config = StreamConfig::default();
        assert_eq!(config.buffer_size, 100);
        assert_eq!(config.chunk_timeout_ms, 5000);
        assert!(config.enable_backpressure);
    }
    
    #[test]
    fn test_stream_collector_new() {
        let collector = StreamCollector::new();
        assert_eq!(collector.chunk_count(), 0);
        assert_eq!(collector.total_bytes(), 0);
    }
    
    #[test]
    fn test_stream_collector_add_chunk() {
        let mut collector = StreamCollector::new();
        collector.add_chunk("Hello ".to_string());
        collector.add_chunk("World".to_string());
        
        assert_eq!(collector.chunk_count(), 2);
        assert_eq!(collector.total_bytes(), 11);
        assert_eq!(collector.get_full_text(), "Hello World");
    }
    
    #[test]
    fn test_stream_collector_clear() {
        let mut collector = StreamCollector::new();
        collector.add_chunk("test".to_string());
        collector.clear();
        
        assert_eq!(collector.chunk_count(), 0);
        assert_eq!(collector.total_bytes(), 0);
    }
    
    #[test]
    fn test_stream_metrics_new() {
        let metrics = StreamMetrics::new();
        assert_eq!(metrics.chunks_received, 0);
        assert_eq!(metrics.total_bytes, 0);
    }
    
    #[test]
    fn test_stream_metrics_calculate() {
        let mut metrics = StreamMetrics {
            chunks_received: 10,
            total_bytes: 1000,
            duration_ms: 1000,
            avg_chunk_size: 0.0,
            throughput_bytes_per_sec: 0.0,
        };
        
        metrics.calculate();
        
        assert_eq!(metrics.avg_chunk_size, 100.0);
        assert_eq!(metrics.throughput_bytes_per_sec, 1000.0);
    }
}
