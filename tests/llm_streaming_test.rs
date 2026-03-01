use strike_security::llm::streaming::{StreamCollector, StreamConfig, StreamMetrics, StreamProcessor};

#[test]
fn test_stream_config_creation() {
    let config = StreamConfig::default();
    
    assert_eq!(config.buffer_size, 100);
    assert_eq!(config.chunk_timeout_ms, 5000);
    assert!(config.enable_backpressure);
}

#[test]
fn test_stream_config_custom() {
    let config = StreamConfig {
        buffer_size: 200,
        chunk_timeout_ms: 10000,
        enable_backpressure: false,
    };
    
    assert_eq!(config.buffer_size, 200);
    assert_eq!(config.chunk_timeout_ms, 10000);
    assert!(!config.enable_backpressure);
}

#[test]
fn test_stream_collector_empty() {
    let collector = StreamCollector::new();
    
    assert_eq!(collector.chunk_count(), 0);
    assert_eq!(collector.total_bytes(), 0);
    assert_eq!(collector.get_full_text(), "");
}

#[test]
fn test_stream_collector_single_chunk() {
    let mut collector = StreamCollector::new();
    collector.add_chunk("Hello World".to_string());
    
    assert_eq!(collector.chunk_count(), 1);
    assert_eq!(collector.total_bytes(), 11);
    assert_eq!(collector.get_full_text(), "Hello World");
}

#[test]
fn test_stream_collector_multiple_chunks() {
    let mut collector = StreamCollector::new();
    collector.add_chunk("The ".to_string());
    collector.add_chunk("quick ".to_string());
    collector.add_chunk("brown ".to_string());
    collector.add_chunk("fox".to_string());
    
    assert_eq!(collector.chunk_count(), 4);
    assert_eq!(collector.get_full_text(), "The quick brown fox");
}

#[test]
fn test_stream_collector_clear() {
    let mut collector = StreamCollector::new();
    collector.add_chunk("test".to_string());
    collector.add_chunk("data".to_string());
    
    assert_eq!(collector.chunk_count(), 2);
    
    collector.clear();
    
    assert_eq!(collector.chunk_count(), 0);
    assert_eq!(collector.total_bytes(), 0);
    assert_eq!(collector.get_full_text(), "");
}

#[test]
fn test_stream_collector_unicode() {
    let mut collector = StreamCollector::new();
    collector.add_chunk("Hello ".to_string());
    collector.add_chunk("世界 ".to_string());
    collector.add_chunk("🌍".to_string());
    
    assert_eq!(collector.chunk_count(), 3);
    assert!(collector.get_full_text().contains("世界"));
    assert!(collector.get_full_text().contains("🌍"));
}

#[test]
fn test_stream_collector_empty_chunks() {
    let mut collector = StreamCollector::new();
    collector.add_chunk("".to_string());
    collector.add_chunk("test".to_string());
    collector.add_chunk("".to_string());
    
    assert_eq!(collector.chunk_count(), 3);
    assert_eq!(collector.get_full_text(), "test");
}

#[test]
fn test_stream_metrics_new() {
    let metrics = StreamMetrics::new();
    
    assert_eq!(metrics.chunks_received, 0);
    assert_eq!(metrics.total_bytes, 0);
    assert_eq!(metrics.duration_ms, 0);
    assert_eq!(metrics.avg_chunk_size, 0.0);
    assert_eq!(metrics.throughput_bytes_per_sec, 0.0);
}

#[test]
fn test_stream_metrics_calculate_avg_chunk_size() {
    let mut metrics = StreamMetrics {
        chunks_received: 10,
        total_bytes: 1000,
        duration_ms: 0,
        avg_chunk_size: 0.0,
        throughput_bytes_per_sec: 0.0,
    };
    
    metrics.calculate();
    
    assert_eq!(metrics.avg_chunk_size, 100.0);
}

#[test]
fn test_stream_metrics_calculate_throughput() {
    let mut metrics = StreamMetrics {
        chunks_received: 0,
        total_bytes: 5000,
        duration_ms: 1000,
        avg_chunk_size: 0.0,
        throughput_bytes_per_sec: 0.0,
    };
    
    metrics.calculate();
    
    assert_eq!(metrics.throughput_bytes_per_sec, 5000.0);
}

#[test]
fn test_stream_metrics_zero_chunks() {
    let mut metrics = StreamMetrics::new();
    metrics.calculate();
    
    assert_eq!(metrics.avg_chunk_size, 0.0);
}

#[test]
fn test_stream_metrics_zero_duration() {
    let mut metrics = StreamMetrics {
        chunks_received: 10,
        total_bytes: 1000,
        duration_ms: 0,
        avg_chunk_size: 0.0,
        throughput_bytes_per_sec: 0.0,
    };
    
    metrics.calculate();
    
    assert_eq!(metrics.throughput_bytes_per_sec, 0.0);
}

#[test]
fn test_stream_processor_creation() {
    let config = StreamConfig::default();
    let processor = StreamProcessor::new(config);
    
    assert!(true);
}

#[tokio::test]
async fn test_stream_processor_create_channel() {
    let config = StreamConfig::default();
    let processor = StreamProcessor::new(config);
    
    let (tx, _stream) = processor.create_channel_stream();
    
    assert!(tx.send(Ok("test".to_string())).await.is_ok());
}

#[test]
fn test_stream_collector_large_text() {
    let mut collector = StreamCollector::new();
    
    for i in 0..100 {
        collector.add_chunk(format!("chunk{} ", i));
    }
    
    assert_eq!(collector.chunk_count(), 100);
    assert!(collector.total_bytes() > 0);
}

#[test]
fn test_stream_metrics_realistic_scenario() {
    let mut metrics = StreamMetrics {
        chunks_received: 50,
        total_bytes: 10000,
        duration_ms: 2000,
        avg_chunk_size: 0.0,
        throughput_bytes_per_sec: 0.0,
    };
    
    metrics.calculate();
    
    assert_eq!(metrics.avg_chunk_size, 200.0);
    assert_eq!(metrics.throughput_bytes_per_sec, 5000.0);
}

#[test]
fn test_stream_collector_default() {
    let collector = StreamCollector::default();
    
    assert_eq!(collector.chunk_count(), 0);
}

#[test]
fn test_stream_metrics_default() {
    let metrics = StreamMetrics::default();
    
    assert_eq!(metrics.chunks_received, 0);
}
