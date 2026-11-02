//! Performance measurement and optimization utilities
//!
//! Provides functionality for:
//! - Timing individual operations
//! - Collecting performance metrics
//! - Profiling conversion pipeline stages

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// A simple timer for measuring operation duration
pub struct Timer {
    start: Instant,
    name: String,
}

impl Timer {
    /// Start a new timer with a name
    pub fn start(name: impl Into<String>) -> Self {
        let name = name.into();
        debug!("Starting timer: {}", name);
        Self {
            start: Instant::now(),
            name,
        }
    }

    /// Stop the timer and return elapsed time
    pub fn stop(self) -> Duration {
        let elapsed = self.start.elapsed();
        info!("{} took {:?}", self.name, elapsed);
        elapsed
    }

    /// Get elapsed time without stopping
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

/// Collected metrics from a conversion run
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConversionMetrics {
    /// Total time for the conversion
    pub total_time_ms: u64,
    /// Time spent loading files
    pub load_time_ms: u64,
    /// Time spent encoding with FFmpeg
    pub encode_time_ms: u64,
    /// Time spent writing metadata
    pub metadata_time_ms: u64,
    /// Time spent embedding artwork
    pub artwork_time_ms: u64,
    /// Time spent embedding chapters
    pub chapters_time_ms: u64,
    /// Time spent verifying output
    pub verify_time_ms: u64,
    /// Input file count
    pub input_file_count: usize,
    /// Input total size in bytes
    pub input_size_bytes: u64,
    /// Input total duration in milliseconds
    pub input_duration_ms: u64,
    /// Output file size in bytes
    pub output_size_bytes: u64,
    /// Encoding speed multiplier (e.g., 2.5x means 2.5x realtime)
    pub encoding_speed: Option<f32>,
}

impl ConversionMetrics {
    /// Create new metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate encoding speed based on input duration and encode time
    pub fn calculate_speed(&mut self) {
        if self.encode_time_ms > 0 && self.input_duration_ms > 0 {
            self.encoding_speed = Some(self.input_duration_ms as f32 / self.encode_time_ms as f32);
        }
    }

    /// Get a summary of the metrics
    pub fn summary(&self) -> String {
        let mut parts = vec![format!("Total: {:.2}s", self.total_time_ms as f64 / 1000.0)];

        if self.encode_time_ms > 0 {
            parts.push(format!(
                "Encode: {:.2}s",
                self.encode_time_ms as f64 / 1000.0
            ));
        }

        if let Some(speed) = self.encoding_speed {
            parts.push(format!("Speed: {:.1}x", speed));
        }

        parts.join(", ")
    }
}

/// Metrics collector for tracking multiple operations
#[derive(Debug, Clone, Default)]
pub struct MetricsCollector {
    timers: HashMap<String, Duration>,
    current_timer: Option<(String, Instant)>,
}

impl MetricsCollector {
    /// Create a new collector
    pub fn new() -> Self {
        Self::default()
    }

    /// Start timing an operation
    pub fn start(&mut self, name: impl Into<String>) {
        let name = name.into();
        if let Some((prev_name, start)) = self.current_timer.take() {
            self.timers.insert(prev_name, start.elapsed());
        }
        debug!("Starting measurement: {}", name);
        self.current_timer = Some((name, Instant::now()));
    }

    /// Stop the current timer
    pub fn stop(&mut self) {
        if let Some((name, start)) = self.current_timer.take() {
            let elapsed = start.elapsed();
            debug!("{}: {:?}", name, elapsed);
            self.timers.insert(name, elapsed);
        }
    }

    /// Get the duration of a named operation
    pub fn get(&self, name: &str) -> Option<Duration> {
        self.timers.get(name).copied()
    }

    /// Get all recorded timings
    pub fn all(&self) -> &HashMap<String, Duration> {
        &self.timers
    }

    /// Get total time across all operations
    pub fn total(&self) -> Duration {
        self.timers.values().sum()
    }

    /// Convert to ConversionMetrics
    pub fn to_metrics(&self) -> ConversionMetrics {
        let get_ms =
            |name: &str| -> u64 { self.get(name).map(|d| d.as_millis() as u64).unwrap_or(0) };

        ConversionMetrics {
            total_time_ms: self.total().as_millis() as u64,
            load_time_ms: get_ms("load"),
            encode_time_ms: get_ms("encode"),
            metadata_time_ms: get_ms("metadata"),
            artwork_time_ms: get_ms("artwork"),
            chapters_time_ms: get_ms("chapters"),
            verify_time_ms: get_ms("verify"),
            ..Default::default()
        }
    }

    /// Log all timings
    pub fn log_summary(&self) {
        info!("Performance summary:");
        for (name, duration) in &self.timers {
            info!("  {}: {:?}", name, duration);
        }
        info!("  Total: {:?}", self.total());
    }
}

/// Macro for timing a block of code
#[macro_export]
macro_rules! measure_time {
    ($name:expr, $block:expr) => {{
        let _timer = $crate::perf::Timer::start($name);
        let result = $block;
        _timer.stop();
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_timer() {
        let timer = Timer::start("test");
        sleep(Duration::from_millis(10));
        let elapsed = timer.stop();
        assert!(elapsed.as_millis() >= 10);
    }

    #[test]
    fn test_metrics_collector() {
        let mut collector = MetricsCollector::new();

        collector.start("op1");
        sleep(Duration::from_millis(5));
        collector.stop();

        collector.start("op2");
        sleep(Duration::from_millis(5));
        collector.stop();

        assert!(collector.get("op1").is_some());
        assert!(collector.get("op2").is_some());
        assert!(collector.total().as_millis() >= 10);
    }

    #[test]
    fn test_encoding_speed() {
        let mut metrics = ConversionMetrics {
            input_duration_ms: 60000, // 1 minute
            encode_time_ms: 30000,    // 30 seconds
            ..Default::default()
        };

        metrics.calculate_speed();
        assert!((metrics.encoding_speed.unwrap() - 2.0).abs() < 0.01);
    }
}
