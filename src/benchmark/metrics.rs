use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMetrics {
    pub precision: f32,
    pub recall: f32,
    pub f1_score: f32,
    pub true_positives: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
    pub detection_rate: f32,
}

pub struct MetricsCalculator;

impl MetricsCalculator {
    pub fn new() -> Self {
        Self
    }
    
    pub fn calculate(&self, expected: usize, detected: usize, correct: usize) -> BenchmarkMetrics {
        let true_positives = correct;
        let false_positives = detected.saturating_sub(correct);
        let false_negatives = expected.saturating_sub(correct);
        
        let precision = if detected > 0 {
            true_positives as f32 / detected as f32
        } else {
            0.0
        };
        
        let recall = if expected > 0 {
            true_positives as f32 / expected as f32
        } else {
            0.0
        };
        
        let f1_score = if precision + recall > 0.0 {
            2.0 * (precision * recall) / (precision + recall)
        } else {
            0.0
        };
        
        let detection_rate = recall * 100.0;
        
        BenchmarkMetrics {
            precision,
            recall,
            f1_score,
            true_positives,
            false_positives,
            false_negatives,
            detection_rate,
        }
    }
    
    pub fn calculate_by_class(&self, expected: &[(String, bool)], detected: &[(String, bool)]) -> BenchmarkMetrics {
        let mut tp = 0;
        let mut fp = 0;
        let mut fn_count = 0;
        
        for (class, is_expected) in expected {
            let is_detected = detected.iter().any(|(c, d)| c == class && *d);
            
            if *is_expected && is_detected {
                tp += 1;
            } else if *is_expected && !is_detected {
                fn_count += 1;
            }
        }
        
        for (class, is_detected) in detected {
            let is_expected = expected.iter().any(|(c, e)| c == class && *e);
            
            if *is_detected && !is_expected {
                fp += 1;
            }
        }
        
        let total_detected = tp + fp;
        let total_expected = tp + fn_count;
        
        self.calculate(total_expected, total_detected, tp)
    }
}

impl Default for MetricsCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_calculator_creation() {
        let calculator = MetricsCalculator::new();
        assert_eq!(calculator, MetricsCalculator);
    }
    
    #[test]
    fn test_calculate_perfect_detection() {
        let calculator = MetricsCalculator::new();
        let metrics = calculator.calculate(10, 10, 10);
        
        assert_eq!(metrics.precision, 1.0);
        assert_eq!(metrics.recall, 1.0);
        assert_eq!(metrics.f1_score, 1.0);
        assert_eq!(metrics.true_positives, 10);
        assert_eq!(metrics.false_positives, 0);
        assert_eq!(metrics.false_negatives, 0);
        assert_eq!(metrics.detection_rate, 100.0);
    }
    
    #[test]
    fn test_calculate_with_false_positives() {
        let calculator = MetricsCalculator::new();
        let metrics = calculator.calculate(10, 15, 10);
        
        assert_eq!(metrics.true_positives, 10);
        assert_eq!(metrics.false_positives, 5);
        assert_eq!(metrics.false_negatives, 0);
        assert!((metrics.precision - 0.666).abs() < 0.01);
        assert_eq!(metrics.recall, 1.0);
    }
    
    #[test]
    fn test_calculate_with_false_negatives() {
        let calculator = MetricsCalculator::new();
        let metrics = calculator.calculate(10, 7, 7);
        
        assert_eq!(metrics.true_positives, 7);
        assert_eq!(metrics.false_positives, 0);
        assert_eq!(metrics.false_negatives, 3);
        assert_eq!(metrics.precision, 1.0);
        assert_eq!(metrics.recall, 0.7);
    }
    
    #[test]
    fn test_calculate_zero_detection() {
        let calculator = MetricsCalculator::new();
        let metrics = calculator.calculate(10, 0, 0);
        
        assert_eq!(metrics.precision, 0.0);
        assert_eq!(metrics.recall, 0.0);
        assert_eq!(metrics.f1_score, 0.0);
    }
    
    #[test]
    fn test_calculate_by_class() {
        let calculator = MetricsCalculator::new();
        
        let expected = vec![
            ("SQLi".to_string(), true),
            ("XSS".to_string(), true),
            ("IDOR".to_string(), true),
        ];
        
        let detected = vec![
            ("SQLi".to_string(), true),
            ("XSS".to_string(), true),
            ("CSRF".to_string(), true),
        ];
        
        let metrics = calculator.calculate_by_class(&expected, &detected);
        
        assert_eq!(metrics.true_positives, 2);
        assert_eq!(metrics.false_positives, 1);
        assert_eq!(metrics.false_negatives, 1);
    }
}
