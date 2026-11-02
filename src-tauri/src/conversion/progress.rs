//! Progress tracking module
//!
//! Provides progress tracking for conversion jobs, including
//! time estimation, smoothing, and reporting.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{debug, trace};

/// Minimum time between progress updates (to avoid flooding)
const MIN_UPDATE_INTERVAL: Duration = Duration::from_millis(500);

/// Number of samples for smoothing speed calculation
const SPEED_SMOOTHING_SAMPLES: usize = 5;

/// Progress information for a conversion job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionProgress {
    /// Job ID
    pub job_id: String,
    /// Current file index being processed
    pub file_index: usize,
    /// Current file name
    pub current_file: String,
    /// Progress within current file (0-100)
    pub file_progress: f32,
    /// Total progress across all files (0-100)
    pub total_progress: f32,
    /// Estimated time remaining in seconds
    pub time_remaining_sec: u32,
    /// Estimated completion time
    pub estimated_finish: DateTime<Utc>,
    /// Current processing speed (e.g., 2.5x realtime)
    pub speed: Option<f32>,
    /// Current stage of processing
    pub stage: ConversionStage,
}

/// Stage of the conversion process
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConversionStage {
    /// Preparing files
    Preparing,
    /// Normalizing audio levels
    Normalizing,
    /// Encoding audio with FFmpeg
    Encoding,
    /// Embedding metadata
    Metadata,
    /// Embedding chapters
    Chapters,
    /// Verifying output
    Verifying,
    /// Finalizing (moving to output location)
    Finalizing,
    /// Completed
    Complete,
}

impl std::fmt::Display for ConversionStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversionStage::Preparing => write!(f, "Preparing"),
            ConversionStage::Normalizing => write!(f, "Normalizing audio"),
            ConversionStage::Encoding => write!(f, "Encoding"),
            ConversionStage::Metadata => write!(f, "Embedding metadata"),
            ConversionStage::Chapters => write!(f, "Embedding chapters"),
            ConversionStage::Verifying => write!(f, "Verifying"),
            ConversionStage::Finalizing => write!(f, "Finalizing"),
            ConversionStage::Complete => write!(f, "Complete"),
        }
    }
}

/// Tracks progress of a conversion job
#[derive(Debug)]
pub struct ProgressTracker {
    /// Job ID
    job_id: String,
    /// Total duration of all input files in milliseconds
    total_duration_ms: u64,
    /// Total number of input files
    total_files: usize,
    /// Current file index
    current_file_index: usize,
    /// Current file name
    current_file_name: String,
    /// Duration of files already processed
    processed_duration_ms: u64,
    /// Current position within current file
    current_position_ms: u64,
    /// Duration of current file
    current_file_duration_ms: u64,
    /// Start time of the conversion
    start_time: Instant,
    /// Last progress update time
    last_update: Instant,
    /// Current processing stage
    stage: ConversionStage,
    /// Speed samples for smoothing
    speed_samples: Vec<f32>,
    /// Last emitted progress (for change detection)
    last_emitted_progress: f32,
    /// Last emitted conversion stage (for immediate stage transition emission)
    last_emitted_stage: Option<ConversionStage>,
    /// Whether we've emitted at least one progress with speed (for ETA)
    has_emitted_with_speed: bool,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new(job_id: String, total_duration_ms: u64, total_files: usize) -> Self {
        let now = Instant::now();
        Self {
            job_id,
            total_duration_ms,
            total_files,
            current_file_index: 0,
            current_file_name: String::new(),
            processed_duration_ms: 0,
            current_position_ms: 0,
            current_file_duration_ms: 0,
            start_time: now,
            last_update: now,
            stage: ConversionStage::Preparing,
            speed_samples: Vec::with_capacity(SPEED_SMOOTHING_SAMPLES),
            last_emitted_progress: 0.0,
            last_emitted_stage: None,
            has_emitted_with_speed: false,
        }
    }

    /// Start tracking (reset timers)
    pub fn start(&mut self) {
        self.start_time = Instant::now();
        self.last_update = self.start_time;
        self.stage = ConversionStage::Encoding;
        debug!("Progress tracker started for job {}", self.job_id);
    }
    
    /// Get total duration in milliseconds
    pub fn total_duration_ms(&self) -> u64 {
        self.total_duration_ms
    }

    /// Set the current file being processed
    pub fn set_current_file(&mut self, index: usize, name: &str, duration_ms: u64) {
        // Add previous file's duration to processed
        if index > self.current_file_index {
            self.processed_duration_ms += self.current_file_duration_ms;
        }

        self.current_file_index = index;
        self.current_file_name = name.to_string();
        self.current_file_duration_ms = duration_ms;
        self.current_position_ms = 0;

        debug!(
            "Processing file {}/{}: {} ({} ms)",
            index + 1,
            self.total_files,
            name,
            duration_ms
        );
    }

    /// Set the current processing stage
    pub fn set_stage(&mut self, stage: ConversionStage) {
        if self.stage != stage {
            debug!("Stage changed: {} -> {}", self.stage, stage);
            self.stage = stage;
        }
    }

    /// Update progress with current position in milliseconds
    pub fn update_position(&mut self, position_ms: u64) {
        self.current_position_ms = position_ms;
    }

    /// Update progress with FFmpeg speed information
    pub fn update_speed(&mut self, speed: f32) {
        if speed > 0.0 {
            self.speed_samples.push(speed);
            if self.speed_samples.len() > SPEED_SMOOTHING_SAMPLES {
                self.speed_samples.remove(0);
            }
        }
    }

    /// Calculate elapsed time since start
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Calculate progress percentage (0-100)
    pub fn progress_percent(&self) -> f32 {
        if self.total_duration_ms == 0 {
            return 0.0;
        }

        let total_processed = self.processed_duration_ms + self.current_position_ms;
        let percent = (total_processed as f64 / self.total_duration_ms as f64) * 100.0;
        percent.clamp(0.0, 100.0) as f32
    }

    /// Calculate file progress percentage (0-100)
    pub fn file_progress_percent(&self) -> f32 {
        if self.current_file_duration_ms == 0 {
            return 0.0;
        }

        let percent =
            (self.current_position_ms as f64 / self.current_file_duration_ms as f64) * 100.0;
        percent.clamp(0.0, 100.0) as f32
    }

    /// Calculate smoothed speed
    pub fn smoothed_speed(&self) -> Option<f32> {
        if self.speed_samples.is_empty() {
            return None;
        }
        let sum: f32 = self.speed_samples.iter().sum();
        Some(sum / self.speed_samples.len() as f32)
    }

    /// Calculate estimated time remaining in seconds
    pub fn time_remaining(&self) -> Duration {
        let percent = self.progress_percent();
        if percent <= 0.0 {
            return Duration::from_secs(0);
        }

        let elapsed = self.elapsed();

        // Use speed-based estimation if available
        if let Some(speed) = self.smoothed_speed() {
            if speed > 0.0 {
                let remaining_duration_ms = self.total_duration_ms
                    .saturating_sub(self.processed_duration_ms + self.current_position_ms);
                let remaining_secs = (remaining_duration_ms as f64 / 1000.0) / speed as f64;
                return Duration::from_secs_f64(remaining_secs.max(0.0));
            }
        }

        // Fallback to simple linear estimation
        if percent >= 100.0 {
            Duration::from_secs(0)
        } else {
            let total_estimated = elapsed.as_secs_f64() * (100.0 / percent as f64);
            let remaining = total_estimated - elapsed.as_secs_f64();
            Duration::from_secs_f64(remaining.max(0.0))
        }
    }

    /// Check if enough time has passed to emit an update
    pub fn should_emit(&self) -> bool {
        // Always emit immediately when the conversion stage changes
        if self.last_emitted_stage != Some(self.stage) {
            return true;
        }

        let elapsed_since_update = self.last_update.elapsed();

        // Always emit first progress with speed data (for ETA display)
        if !self.has_emitted_with_speed && !self.speed_samples.is_empty() {
            return true;
        }

        // Emit if enough time passed OR progress changed by ≥1%
        if elapsed_since_update >= MIN_UPDATE_INTERVAL {
            return true;
        }

        let current_progress = self.progress_percent();
        (current_progress - self.last_emitted_progress).abs() >= 1.0
    }

    /// Get current progress information
    ///
    /// Returns None if not enough time has passed since last update
    /// and progress hasn't changed significantly.
    pub fn get_progress(&mut self) -> Option<ConversionProgress> {
        if !self.should_emit() {
            return None;
        }

        self.last_update = Instant::now();
        let progress = self.progress_percent();
        self.last_emitted_progress = progress;
        self.last_emitted_stage = Some(self.stage);

        // Track that we've emitted with speed data
        if !self.speed_samples.is_empty() {
            self.has_emitted_with_speed = true;
        }

        let time_remaining = self.time_remaining();
        let estimated_finish = Utc::now() + chrono::Duration::from_std(time_remaining).unwrap_or_default();

        Some(ConversionProgress {
            job_id: self.job_id.clone(),
            file_index: self.current_file_index,
            current_file: self.current_file_name.clone(),
            file_progress: self.file_progress_percent(),
            total_progress: progress,
            time_remaining_sec: time_remaining.as_secs() as u32,
            estimated_finish,
            speed: self.smoothed_speed(),
            stage: self.stage,
        })
    }

    /// Force emit current progress (ignores timing)
    pub fn get_progress_forced(&mut self) -> ConversionProgress {
        self.last_update = Instant::now();
        let progress = self.progress_percent();
        self.last_emitted_progress = progress;
        self.last_emitted_stage = Some(self.stage);

        if !self.speed_samples.is_empty() {
            self.has_emitted_with_speed = true;
        }

        let time_remaining = self.time_remaining();
        let estimated_finish = Utc::now() + chrono::Duration::from_std(time_remaining).unwrap_or_default();

        ConversionProgress {
            job_id: self.job_id.clone(),
            file_index: self.current_file_index,
            current_file: self.current_file_name.clone(),
            file_progress: self.file_progress_percent(),
            total_progress: progress,
            time_remaining_sec: time_remaining.as_secs() as u32,
            estimated_finish,
            speed: self.smoothed_speed(),
            stage: self.stage,
        }
    }

    /// Mark as complete
    pub fn complete(&mut self) {
        self.stage = ConversionStage::Complete;
        self.processed_duration_ms = self.total_duration_ms;
        self.current_position_ms = self.current_file_duration_ms;
        debug!(
            "Conversion completed in {:?} for job {}",
            self.elapsed(),
            self.job_id
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_tracker_creation() {
        let tracker = ProgressTracker::new("test-job".to_string(), 60000, 3);
        assert_eq!(tracker.progress_percent(), 0.0);
        assert_eq!(tracker.file_progress_percent(), 0.0);
    }

    #[test]
    fn test_progress_tracker_update() {
        let mut tracker = ProgressTracker::new("test-job".to_string(), 60000, 3);
        tracker.start();
        
        // Set first file (20 seconds)
        tracker.set_current_file(0, "file1.mp3", 20000);
        
        // Update to 10 seconds into first file
        tracker.update_position(10000);
        
        // 10000 / 60000 = ~16.67%
        assert!((tracker.progress_percent() - 16.67).abs() < 0.1);
        
        // 10000 / 20000 = 50%
        assert_eq!(tracker.file_progress_percent(), 50.0);
    }

    #[test]
    fn test_progress_tracker_file_transition() {
        let mut tracker = ProgressTracker::new("test-job".to_string(), 60000, 3);
        tracker.start();
        
        // Complete first file (20 seconds)
        tracker.set_current_file(0, "file1.mp3", 20000);
        tracker.update_position(20000);
        
        // Move to second file
        tracker.set_current_file(1, "file2.mp3", 20000);
        tracker.update_position(10000);
        
        // 20000 + 10000 = 30000 / 60000 = 50%
        assert_eq!(tracker.progress_percent(), 50.0);
    }

    #[test]
    fn test_progress_tracker_speed_smoothing() {
        let mut tracker = ProgressTracker::new("test-job".to_string(), 60000, 1);
        
        tracker.update_speed(2.0);
        tracker.update_speed(2.5);
        tracker.update_speed(2.0);
        
        let speed = tracker.smoothed_speed().unwrap();
        assert!((speed - 2.167).abs() < 0.01);
    }

    #[test]
    fn test_conversion_stage_display() {
        assert_eq!(ConversionStage::Encoding.to_string(), "Encoding");
        assert_eq!(ConversionStage::Metadata.to_string(), "Embedding metadata");
        assert_eq!(ConversionStage::Complete.to_string(), "Complete");
    }

    #[test]
    fn test_progress_tracker_immediate_stage_transitions() {
        let mut tracker = ProgressTracker::new("test-job".to_string(), 60000, 1);
        tracker.start();

        let stages = [
            ConversionStage::Preparing,
            ConversionStage::Normalizing,
            ConversionStage::Encoding,
            ConversionStage::Chapters,
            ConversionStage::Metadata,
            ConversionStage::Verifying,
            ConversionStage::Finalizing,
            ConversionStage::Complete,
        ];

        for stage in stages {
            tracker.set_stage(stage);
            let emitted = tracker.get_progress().expect("Stage transition must emit immediately");
            assert_eq!(emitted.stage, stage);
        }
    }
}
