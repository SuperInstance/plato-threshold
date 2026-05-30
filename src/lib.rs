use serde::{Deserialize, Serialize};

/// A min/max range for thresholds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThresholdRange {
    pub min: f64,
    pub max: f64,
}

impl ThresholdRange {
    pub fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }

    pub fn contains(&self, value: f64) -> bool {
        value >= self.min && value <= self.max
    }
}

/// Fixed thresholds per sensor type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StaticThreshold {
    pub range: ThresholdRange,
}

impl StaticThreshold {
    pub fn new(min: f64, max: f64) -> Self {
        Self {
            range: ThresholdRange::new(min, max),
        }
    }

    pub fn check(&self, value: f64) -> ThresholdResult {
        if value.is_nan() {
            return ThresholdResult {
                value,
                within_range: false,
                distance_from_range: f64::NAN,
                confidence: 0.0,
            };
        }
        let within = self.range.contains(value);
        let distance = if within {
            0.0
        } else if value < self.range.min {
            self.range.min - value
        } else {
            value - self.range.max
        };
        ThresholdResult {
            value,
            within_range: within,
            distance_from_range: distance,
            confidence: 1.0,
        }
    }
}

/// Method used for adaptive threshold calculation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThresholdMethod {
    Static,
    MovingAverage { window: usize },
    ExponentialSmoothing { alpha: f64 },
    Percentile { lower: f64, upper: f64 },
}

/// Configuration for adaptive thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdConfig {
    pub method: ThresholdMethod,
    pub warmup_samples: usize,
    pub sensitivity: f64,
}

impl Default for ThresholdConfig {
    fn default() -> Self {
        Self {
            method: ThresholdMethod::MovingAverage { window: 10 },
            warmup_samples: 5,
            sensitivity: 2.0,
        }
    }
}

/// Result of a threshold check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThresholdResult {
    pub value: f64,
    pub within_range: bool,
    pub distance_from_range: f64,
    pub confidence: f64,
}

/// Adaptive threshold that learns from historical data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveThreshold {
    pub base: ThresholdRange,
    pub std_dev_multiplier: f64,
    pub history: Vec<f64>,
    pub config: ThresholdConfig,
}

impl AdaptiveThreshold {
    pub fn new(config: ThresholdConfig) -> Self {
        Self {
            base: ThresholdRange::new(f64::NEG_INFINITY, f64::INFINITY),
            std_dev_multiplier: config.sensitivity,
            history: Vec::new(),
            config,
        }
    }

    pub fn is_warmed_up(&self) -> bool {
        self.history.len() >= self.config.warmup_samples
    }

    pub fn update(&mut self, value: f64) {
        if !value.is_nan() {
            self.history.push(value);
        }
    }

    pub fn current_range(&self) -> ThresholdRange {
        if !self.is_warmed_up() {
            return ThresholdRange::new(f64::NEG_INFINITY, f64::INFINITY);
        }

        let data: Vec<f64> = self.history.iter().copied().filter(|v| !v.is_nan()).collect();
        if data.is_empty() {
            return ThresholdRange::new(f64::NEG_INFINITY, f64::INFINITY);
        }

        let (center, spread) = match &self.config.method {
            ThresholdMethod::Static => {
                let mean = data.iter().sum::<f64>() / data.len() as f64;
                let std_dev = compute_std_dev(&data, mean);
                (mean, std_dev)
            }
            ThresholdMethod::MovingAverage { window } => {
                let smoothed = moving_average(&data, *window);
                let last = *smoothed.last().unwrap_or(&0.0);
                let std_dev = compute_std_dev(&data, last);
                (last, std_dev)
            }
            ThresholdMethod::ExponentialSmoothing { alpha } => {
                let smoothed = exponential_smoothing(&data, *alpha);
                let last = *smoothed.last().unwrap_or(&0.0);
                let std_dev = compute_std_dev(&data, last);
                (last, std_dev)
            }
            ThresholdMethod::Percentile { lower, upper } => {
                let lo = percentile(&data, *lower);
                let hi = percentile(&data, *upper);
                return ThresholdRange::new(lo, hi);
            }
        };

        let margin = spread * self.std_dev_multiplier;
        ThresholdRange::new(center - margin, center + margin)
    }

    pub fn check(&self, value: f64) -> ThresholdResult {
        if value.is_nan() {
            return ThresholdResult {
                value,
                within_range: false,
                distance_from_range: f64::NAN,
                confidence: 0.0,
            };
        }

        let range = self.current_range();
        let within = range.contains(value);
        let distance = if within {
            0.0
        } else if value < range.min {
            range.min - value
        } else {
            value - range.max
        };

        let confidence = if self.is_warmed_up() {
            1.0_f64.min(self.history.len() as f64 / (self.config.warmup_samples * 3) as f64)
        } else {
            0.0
        };

        ThresholdResult {
            value,
            within_range: within,
            distance_from_range: distance,
            confidence,
        }
    }
}

// --- Free functions ---

pub fn moving_average(data: &[f64], window: usize) -> Vec<f64> {
    if window == 0 || data.is_empty() {
        return data.to_vec();
    }
    let mut result = Vec::with_capacity(data.len());
    for i in 0..data.len() {
        let start = i.saturating_sub(window - 1);
        let window_slice = &data[start..=i];
        let avg = window_slice.iter().sum::<f64>() / window_slice.len() as f64;
        result.push(avg);
    }
    result
}

pub fn exponential_smoothing(data: &[f64], alpha: f64) -> Vec<f64> {
    if data.is_empty() {
        return Vec::new();
    }
    let alpha = alpha.clamp(0.0, 1.0);
    let mut result = Vec::with_capacity(data.len());
    result.push(data[0]);
    for i in 1..data.len() {
        let prev = result[i - 1];
        let smoothed = alpha * data[i] + (1.0 - alpha) * prev;
        result.push(smoothed);
    }
    result
}

pub fn percentile(data: &[f64], p: f64) -> f64 {
    if data.is_empty() {
        return f64::NAN;
    }
    let p = p.clamp(0.0, 100.0);
    let mut sorted: Vec<f64> = data.iter().copied().filter(|v| !v.is_nan()).collect();
    if sorted.is_empty() {
        return f64::NAN;
    }
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    if sorted.len() == 1 {
        return sorted[0];
    }

    let idx = (p / 100.0) * (sorted.len() - 1) as f64;
    let lower = idx.floor() as usize;
    let upper = idx.ceil() as usize;
    let lower = lower.min(sorted.len() - 1);
    let upper = upper.min(sorted.len() - 1);
    let frac = idx - lower as f64;
    sorted[lower] * (1.0 - frac) + sorted[upper] * frac
}

fn compute_std_dev(data: &[f64], mean: f64) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let variance: f64 =
        data.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / data.len() as f64;
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- StaticThreshold tests ---

    #[test]
    fn static_threshold_in_range() {
        let t = StaticThreshold::new(0.0, 100.0);
        let r = t.check(50.0);
        assert!(r.within_range);
        assert_eq!(r.distance_from_range, 0.0);
    }

    #[test]
    fn static_threshold_above_range() {
        let t = StaticThreshold::new(0.0, 100.0);
        let r = t.check(150.0);
        assert!(!r.within_range);
        assert!((r.distance_from_range - 50.0).abs() < 1e-9);
    }

    #[test]
    fn static_threshold_below_range() {
        let t = StaticThreshold::new(0.0, 100.0);
        let r = t.check(-10.0);
        assert!(!r.within_range);
        assert!((r.distance_from_range - 10.0).abs() < 1e-9);
    }

    #[test]
    fn static_threshold_boundary() {
        let t = StaticThreshold::new(0.0, 100.0);
        assert!(t.check(0.0).within_range);
        assert!(t.check(100.0).within_range);
    }

    // --- Moving average ---

    #[test]
    fn moving_average_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = moving_average(&data, 3);
        assert!((result[0] - 1.0).abs() < 1e-9);
        assert!((result[2] - 2.0).abs() < 1e-9);
        assert!((result[4] - 4.0).abs() < 1e-9);
    }

    #[test]
    fn moving_average_window_one() {
        let data = vec![1.0, 2.0, 3.0];
        let result = moving_average(&data, 1);
        assert_eq!(result, data);
    }

    #[test]
    fn moving_average_empty() {
        let data: Vec<f64> = vec![];
        let result = moving_average(&data, 3);
        assert!(result.is_empty());
    }

    // --- Exponential smoothing ---

    #[test]
    fn exponential_smoothing_basic() {
        let data = vec![1.0, 2.0, 3.0];
        let result = exponential_smoothing(&data, 0.5);
        assert!((result[0] - 1.0).abs() < 1e-9);
        assert!((result[1] - 1.5).abs() < 1e-9);
        assert!((result[2] - 2.25).abs() < 1e-9);
    }

    #[test]
    fn exponential_smoothing_alpha_zero() {
        let data = vec![1.0, 2.0, 3.0];
        let result = exponential_smoothing(&data, 0.0);
        assert!((result[0] - 1.0).abs() < 1e-9);
        assert!((result[1] - 1.0).abs() < 1e-9);
        assert!((result[2] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn exponential_smoothing_alpha_one() {
        let data = vec![1.0, 2.0, 3.0];
        let result = exponential_smoothing(&data, 1.0);
        assert_eq!(result, data);
    }

    // --- Percentile ---

    #[test]
    fn percentile_median() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = percentile(&data, 50.0);
        assert!((result - 3.0).abs() < 1e-9);
    }

    #[test]
    fn percentile_min() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = percentile(&data, 0.0);
        assert!((result - 1.0).abs() < 1e-9);
    }

    #[test]
    fn percentile_max() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = percentile(&data, 100.0);
        assert!((result - 5.0).abs() < 1e-9);
    }

    #[test]
    fn percentile_empty() {
        let data: Vec<f64> = vec![];
        let result = percentile(&data, 50.0);
        assert!(result.is_nan());
    }

    // --- AdaptiveThreshold ---

    #[test]
    fn adaptive_threshold_learns_normal_data() {
        let config = ThresholdConfig {
            method: ThresholdMethod::MovingAverage { window: 5 },
            warmup_samples: 3,
            sensitivity: 2.0,
        };
        let mut at = AdaptiveThreshold::new(config);
        for v in &[10.0, 11.0, 10.5, 10.2, 9.8] {
            at.update(*v);
        }
        assert!(at.is_warmed_up());
        // Value near the mean should be in range
        let result = at.check(10.3);
        assert!(result.within_range);
        assert!(result.confidence > 0.0);
    }

    #[test]
    fn adaptive_threshold_detects_anomaly() {
        let config = ThresholdConfig {
            method: ThresholdMethod::MovingAverage { window: 5 },
            warmup_samples: 3,
            sensitivity: 1.0, // tight
        };
        let mut at = AdaptiveThreshold::new(config);
        for v in &[10.0, 10.1, 9.9, 10.0, 10.1] {
            at.update(*v);
        }
        // Way out of range
        let result = at.check(100.0);
        assert!(!result.within_range);
    }

    #[test]
    fn adaptive_threshold_warmup_period() {
        let config = ThresholdConfig {
            method: ThresholdMethod::MovingAverage { window: 3 },
            warmup_samples: 5,
            sensitivity: 2.0,
        };
        let mut at = AdaptiveThreshold::new(config);
        assert!(!at.is_warmed_up());
        at.update(10.0);
        at.update(10.0);
        at.update(10.0);
        at.update(10.0);
        assert!(!at.is_warmed_up());
        at.update(10.0);
        assert!(at.is_warmed_up());
    }

    #[test]
    fn adaptive_threshold_not_warmed_accepts_anything() {
        let config = ThresholdConfig {
            method: ThresholdMethod::MovingAverage { window: 3 },
            warmup_samples: 10,
            sensitivity: 2.0,
        };
        let at = AdaptiveThreshold::new(config);
        let result = at.check(99999.0);
        assert!(result.within_range); // range is [-inf, inf] before warmup
        assert_eq!(result.confidence, 0.0);
    }

    // --- Sensitivity ---

    #[test]
    fn tighter_sensitivity_narrows_range() {
        let config_tight = ThresholdConfig {
            method: ThresholdMethod::MovingAverage { window: 5 },
            warmup_samples: 3,
            sensitivity: 0.5,
        };
        let config_loose = ThresholdConfig {
            method: ThresholdMethod::MovingAverage { window: 5 },
            warmup_samples: 3,
            sensitivity: 3.0,
        };
        let data = vec![10.0, 11.0, 9.0, 10.5, 9.5];
        let mut at_tight = AdaptiveThreshold::new(config_tight);
        let mut at_loose = AdaptiveThreshold::new(config_loose);
        for v in &data {
            at_tight.update(*v);
            at_loose.update(*v);
        }
        let range_tight = at_tight.current_range();
        let range_loose = at_loose.current_range();
        assert!(range_tight.max - range_tight.min < range_loose.max - range_loose.min);
    }

    // --- Edge cases ---

    #[test]
    fn constant_data() {
        let config = ThresholdConfig {
            method: ThresholdMethod::MovingAverage { window: 3 },
            warmup_samples: 2,
            sensitivity: 2.0,
        };
        let mut at = AdaptiveThreshold::new(config);
        for _ in 0..5 {
            at.update(42.0);
        }
        // With zero std dev, range should be exactly [42, 42]
        let range = at.current_range();
        assert!((range.min - 42.0).abs() < 1e-9);
        assert!((range.max - 42.0).abs() < 1e-9);
    }

    #[test]
    fn single_sample() {
        let config = ThresholdConfig {
            method: ThresholdMethod::MovingAverage { window: 3 },
            warmup_samples: 1,
            sensitivity: 2.0,
        };
        let mut at = AdaptiveThreshold::new(config);
        at.update(10.0);
        assert!(at.is_warmed_up());
        let range = at.current_range();
        assert!((range.min - 10.0).abs() < 1e-9);
        assert!((range.max - 10.0).abs() < 1e-9);
    }

    #[test]
    fn nan_handling_static() {
        let t = StaticThreshold::new(0.0, 100.0);
        let r = t.check(f64::NAN);
        assert!(!r.within_range);
        assert!(r.distance_from_range.is_nan());
    }

    #[test]
    fn nan_handling_adaptive() {
        let config = ThresholdConfig {
            method: ThresholdMethod::MovingAverage { window: 3 },
            warmup_samples: 2,
            sensitivity: 2.0,
        };
        let mut at = AdaptiveThreshold::new(config);
        at.update(f64::NAN);
        at.update(f64::NAN);
        assert!(!at.is_warmed_up()); // NaN samples don't count toward history for warmup... actually they do get pushed
        // Let's just check NaN check
        let r = at.check(f64::NAN);
        assert!(!r.within_range);
    }

    #[test]
    fn all_same_value_percentile() {
        let data = vec![5.0; 10];
        let result = percentile(&data, 50.0);
        assert!((result - 5.0).abs() < 1e-9);
    }

    // --- Serialization ---

    #[test]
    fn serde_threshold_range() {
        let range = ThresholdRange::new(1.0, 10.0);
        let json = serde_json::to_string(&range).unwrap();
        let deserialized: ThresholdRange = serde_json::from_str(&json).unwrap();
        assert_eq!(range, deserialized);
    }

    #[test]
    fn serde_adaptive_threshold() {
        let config = ThresholdConfig {
            method: ThresholdMethod::ExponentialSmoothing { alpha: 0.3 },
            warmup_samples: 5,
            sensitivity: 1.5,
        };
        let mut at = AdaptiveThreshold::new(config);
        at.base = ThresholdRange::new(0.0, 100.0);
        at.update(10.0);
        at.update(11.0);
        at.update(9.0);
        let json = serde_json::to_string(&at).unwrap();
        let deserialized: AdaptiveThreshold = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.history.len(), 3);
        assert!((deserialized.history[0] - 10.0).abs() < 1e-9);
        assert!((deserialized.base.min - 0.0).abs() < 1e-9);
    }

    #[test]
    fn serde_threshold_result() {
        let result = ThresholdResult {
            value: 42.0,
            within_range: true,
            distance_from_range: 0.0,
            confidence: 0.95,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ThresholdResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result, deserialized);
    }
}
