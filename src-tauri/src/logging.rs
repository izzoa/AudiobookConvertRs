//! Logging configuration for AudiobookConvertRs
//!
//! Sets up structured logging using the tracing crate with:
//! - Console output for development
//! - File output for debugging
//! - Configurable log levels

use crate::config::{get_settings, LogLevel};
use crate::error::AppResult;
use std::path::PathBuf;
use tracing::Level;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Initialize the logging system
pub fn init_logging() -> AppResult<()> {
    let settings = get_settings();
    let log_level = settings.log_level;

    // Create filter based on settings
    let filter = create_env_filter(log_level);

    // Console layer with colored output
    let console_layer = fmt::layer()
        .with_target(true)
        .with_level(true)
        .with_thread_ids(false)
        .with_file(log_level == LogLevel::Debug || log_level == LogLevel::Trace)
        .with_line_number(log_level == LogLevel::Debug || log_level == LogLevel::Trace)
        .with_span_events(if log_level == LogLevel::Trace {
            FmtSpan::FULL
        } else {
            FmtSpan::NONE
        });

    // Initialize subscriber
    tracing_subscriber::registry()
        .with(filter)
        .with(console_layer)
        .init();

    tracing::info!("Logging initialized at {} level", log_level.as_str());

    Ok(())
}

/// Create an environment filter based on log level
fn create_env_filter(level: LogLevel) -> EnvFilter {
    let level_str = match level {
        LogLevel::Error => "error",
        LogLevel::Warn => "warn",
        LogLevel::Info => "info",
        LogLevel::Debug => "debug",
        LogLevel::Trace => "trace",
    };

    // Allow override from RUST_LOG environment variable
    EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        // Set default levels for different modules
        EnvFilter::new(format!(
            "audiobookconvert_rs={},symphonia=warn,tauri=info,{}",
            level_str, level_str
        ))
    })
}

/// Get the log file path
pub fn get_log_file_path() -> AppResult<PathBuf> {
    let config_dir = crate::config::UserSettings::config_dir()?;
    Ok(config_dir.join("audiobookconvert.log"))
}

/// Macro for timing operations
#[macro_export]
macro_rules! timed {
    ($name:expr, $block:expr) => {{
        let start = std::time::Instant::now();
        let result = $block;
        let elapsed = start.elapsed();
        tracing::debug!("{} completed in {:?}", $name, elapsed);
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_env_filter() {
        let filter = create_env_filter(LogLevel::Info);
        // Just verify it doesn't panic
        assert!(filter.max_level_hint().is_some());
    }
}
