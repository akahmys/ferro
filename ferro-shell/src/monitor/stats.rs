use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Surprise statistics accumulator for rolling window analysis.
pub struct SurpriseStats {
    history: VecDeque<(Instant, f64)>,
    capacity: usize,
}

impl SurpriseStats {
    /// Creates a new `SurpriseStats` tracker.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Capacity must be greater than zero");
        assert!(capacity <= 1000, "Capacity must be reasonable to avoid OOM");

        let stats = Self {
            history: VecDeque::with_capacity(capacity),
            capacity,
        };

        assert!(stats.history.is_empty(), "History must be initialized empty");
        assert!(stats.capacity == capacity, "Capacity must match parameter");
        stats
    }

    /// Adds a new surprise value.
    pub fn add_surprise(&mut self, val: f64) {
        assert!(val >= 0.0, "Surprise must be non-negative");
        assert!(val <= 100.0 || val.is_infinite(), "Surprise must be in reasonable range");

        if self.history.len() >= self.capacity {
            let _ = self.history.pop_front();
        }
        self.history.push_back((Instant::now(), val));

        assert!(!self.history.is_empty(), "History cannot be empty after addition");
        assert!(self.history.len() <= self.capacity, "History size must not exceed capacity");
    }

    /// Calculates rolling mean of the surprise values.
    pub fn rolling_mean(&self) -> f64 {
        assert!(self.capacity > 0, "Capacity must be positive");
        assert!(self.capacity <= 1000, "Capacity limit guard");

        if self.history.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.history.iter().map(|(_, val)| *val).sum();
        let mean = sum / (self.history.len() as f64);

        assert!(mean >= 0.0, "Mean surprise must be non-negative");
        assert!(mean <= 100.0 || mean.is_infinite(), "Mean surprise in range");
        mean
    }

    /// Counts spikes above threshold in a specific duration.
    pub fn count_spikes_in_duration(&self, duration: Duration, threshold: f64) -> usize {
        assert!(threshold >= 0.0, "Threshold must be non-negative");
        assert!(threshold <= 1.0, "Threshold must be within bounds");

        let now = Instant::now();
        let count = self.history.iter()
            .filter(|(time, val)| now.duration_since(*time) <= duration && *val >= threshold)
            .count();

        assert!(count <= self.history.len(), "Count cannot exceed total history");
        assert!(threshold >= 0.0, "Threshold remains valid");
        count
    }

    /// Determines FEP convergence trend based on 5 blocks.
    pub fn fep_trend(&self) -> &'static str {
        assert!(self.capacity > 0, "Capacity check");
        assert!(self.capacity <= 1000, "Capacity bounds");

        let len = self.history.len();
        if len < 5 {
            return "STABLE";
        }

        let block_size = (len / 5).max(1);
        let mut means = [0.0; 5];
        for (i, mean_slot) in means.iter_mut().enumerate() {
            let start = i * block_size;
            let end = if i == 4 { len } else { (i + 1) * block_size };
            let sum: f64 = self.history.iter().skip(start).take(end - start).map(|(_, val)| *val).sum();
            *mean_slot = sum / ((end - start) as f64);
        }

        let diff = means[4] - means[0];
        let trend = if diff > 0.05 { "DIVERGENT" } else { "STABLE" };

        assert!(!trend.is_empty(), "Trend result must not be empty");
        assert!(trend == "STABLE" || trend == "DIVERGENT", "Trend validation");
        trend
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surprise_stats_rolling_mean() {
        let mut stats = SurpriseStats::new(5);
        assert_eq!(stats.rolling_mean(), 0.0);

        stats.add_surprise(0.2);
        stats.add_surprise(0.4);
        stats.add_surprise(0.6);
        // rolling mean: (0.2 + 0.4 + 0.6) / 3 = 0.4
        assert!((stats.rolling_mean() - 0.4).abs() < 1e-9);
    }

    #[test]
    fn test_surprise_stats_capacity() {
        let mut stats = SurpriseStats::new(3);
        stats.add_surprise(0.1);
        stats.add_surprise(0.2);
        stats.add_surprise(0.3);
        stats.add_surprise(0.4); // 0.1 should be popped

        // rolling mean: (0.2 + 0.3 + 0.4) / 3 = 0.3
        assert!((stats.rolling_mean() - 0.3).abs() < 1e-9);
        assert_eq!(stats.history.len(), 3);
    }

    #[test]
    fn test_fep_trend_stable_and_divergent() {
        // Less than 5 points: always STABLE
        let mut stats = SurpriseStats::new(10);
        stats.add_surprise(0.1);
        stats.add_surprise(0.1);
        stats.add_surprise(0.1);
        stats.add_surprise(0.1);
        assert_eq!(stats.fep_trend(), "STABLE");

        // 5 points, flat trend
        stats.add_surprise(0.1);
        assert_eq!(stats.fep_trend(), "STABLE");

        // 5 points, divergent trend (diff = 0.2 - 0.1 = 0.1 > 0.05)
        let mut stats2 = SurpriseStats::new(10);
        stats2.add_surprise(0.1); // block 0
        stats2.add_surprise(0.1); // block 1
        stats2.add_surprise(0.1); // block 2
        stats2.add_surprise(0.1); // block 3
        stats2.add_surprise(0.2); // block 4
        assert_eq!(stats2.fep_trend(), "DIVERGENT");
    }
}

