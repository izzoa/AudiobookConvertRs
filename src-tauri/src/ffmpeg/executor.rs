//! FFmpeg command execution module
//!
//! Provides functionality for executing FFmpeg commands as subprocesses,
//! parsing output for progress tracking, and handling timeouts.

use crate::error::{AppError, AppResult};
use crate::ffmpeg::command_builder::FFmpegCommand;
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{error, info, trace, warn};

/// Output from FFmpeg command execution
#[derive(Debug, Clone)]
pub struct FFmpegOutput {
    /// Exit code from FFmpeg (0 = success)
    pub exit_code: i32,
    /// Standard output content
    pub stdout: String,
    /// Standard error content (FFmpeg writes progress here)
    pub stderr: String,
    /// Duration of the command execution
    pub duration: Duration,
}

impl FFmpegOutput {
    /// Check if the command succeeded
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Progress information parsed from FFmpeg output
#[derive(Debug, Clone, Default)]
pub struct FFmpegProgress {
    /// Current frame number (for video, not used for audio)
    pub frame: Option<u64>,
    /// Frames per second (for video)
    pub fps: Option<f32>,
    /// Current processing time in milliseconds
    pub time_ms: u64,
    /// Current bitrate in kbps
    pub bitrate_kbps: Option<f32>,
    /// Total output size in bytes
    pub total_size: Option<u64>,
    /// Processing speed (e.g., 2.5x means 2.5x realtime)
    pub speed: Option<f32>,
}

impl FFmpegProgress {
    /// Check if this is a valid progress update
    pub fn is_valid(&self) -> bool {
        self.time_ms > 0
    }
}

/// Accumulates FFmpeg progress fields across multiple lines.
///
/// FFmpeg's `-progress pipe:1` outputs each field on its own line:
/// ```text
/// out_time_ms=5000000
/// speed=2.5x
/// progress=continue
/// ```
/// This accumulator collects fields and emits complete progress on `progress=` lines.
#[derive(Debug, Default)]
pub struct FFmpegProgressAccumulator {
    time_ms: u64,
    speed: Option<f32>,
    bitrate_kbps: Option<f32>,
    total_size: Option<u64>,
}

impl FFmpegProgressAccumulator {
    /// Create a new accumulator
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a line and update accumulated state.
    /// Returns Some(progress) when a complete update is ready (on `progress=` lines).
    pub fn parse_line(&mut self, line: &str) -> Option<FFmpegProgress> {
        let line = line.trim();

        // Parse out_time_us or out_time_ms (both are in MICROSECONDS despite names!)
        if line.starts_with("out_time_us=") || line.starts_with("out_time_ms=") {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                if let Ok(us) = parts[1].trim().parse::<u64>() {
                    self.time_ms = us / 1000;
                }
            }
            return None;
        }

        // Parse out_time (HH:MM:SS.ms format)
        if line.starts_with("out_time=") {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                if let Some(ms) = parse_time_to_ms(parts[1].trim()) {
                    self.time_ms = ms;
                }
            }
            return None;
        }

        // Parse speed
        if line.starts_with("speed=") {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                let s = parts[1].trim().trim_end_matches('x');
                if s != "N/A" {
                    if let Ok(speed) = s.parse::<f32>() {
                        self.speed = Some(speed);
                    }
                }
            }
            return None;
        }

        // Parse bitrate
        if line.starts_with("bitrate=") {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                if let Some(kbps) = parse_bitrate(parts[1].trim()) {
                    self.bitrate_kbps = Some(kbps);
                }
            }
            return None;
        }

        // Parse total_size
        if line.starts_with("total_size=") {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                if let Ok(size) = parts[1].trim().parse::<u64>() {
                    self.total_size = Some(size);
                }
            }
            return None;
        }

        // On progress= line, emit accumulated progress and reset
        if line.starts_with("progress=") {
            if self.time_ms > 0 {
                let progress = FFmpegProgress {
                    frame: None,
                    fps: None,
                    time_ms: self.time_ms,
                    bitrate_kbps: self.bitrate_kbps,
                    total_size: self.total_size,
                    speed: self.speed,
                };
                // Don't reset time_ms - it should persist for the next block
                // Speed and bitrate will be updated in subsequent lines
                return Some(progress);
            }
            return None;
        }

        None
    }
}

/// Execute an FFmpeg command and capture output
///
/// Runs FFmpeg as a subprocess and captures stdout/stderr.
/// Progress can be parsed from stderr during or after execution.
pub fn execute_ffmpeg_command(command: &FFmpegCommand) -> AppResult<FFmpegOutput> {
    let args = command.build_args()?;
    let executable = command.executable();

    info!("Executing FFmpeg: {} {}", executable, args.join(" "));
    let start = Instant::now();

    let mut child = Command::new(executable.as_str())
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::FFmpegNotFound
            } else {
                AppError::FFmpegExecutionError {
                    reason: format!("Failed to spawn FFmpeg: {}", e),
                }
            }
        })?;

    // Capture stdout and stderr
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::FFmpegExecutionError {
            reason: "Failed to capture stdout".to_string(),
        })?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::FFmpegExecutionError {
            reason: "Failed to capture stderr".to_string(),
        })?;

    // Read stdout in a separate thread
    let stdout_handle = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        let mut output = String::new();
        for line in reader.lines().map_while(Result::ok) {
            output.push_str(&line);
            output.push('\n');
        }
        output
    });

    // Read stderr in a separate thread (FFmpeg writes progress here)
    let stderr_handle = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        let mut output = String::new();
        for line in reader.lines().map_while(Result::ok) {
            trace!("FFmpeg: {}", line);
            output.push_str(&line);
            output.push('\n');
        }
        output
    });

    // Wait for the process to complete
    let status = child.wait().map_err(|e| AppError::FFmpegExecutionError {
        reason: format!("Failed to wait for FFmpeg: {}", e),
    })?;

    let stdout_content = stdout_handle.join().unwrap_or_default();
    let stderr_content = stderr_handle.join().unwrap_or_default();
    let duration = start.elapsed();

    let exit_code = status.code().unwrap_or(-1);

    if exit_code != 0 {
        error!(
            "FFmpeg failed with exit code {}: {}",
            exit_code, stderr_content
        );
    } else {
        info!("FFmpeg completed successfully in {:?}", duration);
    }

    Ok(FFmpegOutput {
        exit_code,
        stdout: stdout_content,
        stderr: stderr_content,
        duration,
    })
}

/// Execute FFmpeg command with timeout
///
/// Kills the process if it exceeds the specified timeout.
pub fn run_command_with_timeout(
    command: &FFmpegCommand,
    timeout: Duration,
) -> AppResult<FFmpegOutput> {
    let args = command.build_args()?;
    let executable = command.executable();

    info!(
        "Executing FFmpeg with {:?} timeout: {} {}",
        timeout,
        executable,
        args.join(" ")
    );
    let start = Instant::now();

    let mut child = Command::new(executable.as_str())
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::FFmpegNotFound
            } else {
                AppError::FFmpegExecutionError {
                    reason: format!("Failed to spawn FFmpeg: {}", e),
                }
            }
        })?;

    // Check periodically if the process has completed
    let poll_interval = Duration::from_millis(100);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process finished
                let stdout = child.stdout.take();
                let stderr = child.stderr.take();

                let stdout_content = stdout
                    .map(|s| {
                        BufReader::new(s)
                            .lines()
                            .map_while(Result::ok)
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();

                let stderr_content = stderr
                    .map(|s| {
                        BufReader::new(s)
                            .lines()
                            .map_while(Result::ok)
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();

                let exit_code = status.code().unwrap_or(-1);
                let duration = start.elapsed();

                if exit_code != 0 {
                    error!(
                        "FFmpeg failed with exit code {}: {}",
                        exit_code, stderr_content
                    );
                }

                return Ok(FFmpegOutput {
                    exit_code,
                    stdout: stdout_content,
                    stderr: stderr_content,
                    duration,
                });
            }
            Ok(None) => {
                // Process still running
                if start.elapsed() > timeout {
                    warn!("FFmpeg timed out after {:?}, killing process", timeout);
                    let _ = child.kill();
                    return Err(AppError::FFmpegTimeout {
                        seconds: timeout.as_secs(),
                    });
                }
                thread::sleep(poll_interval);
            }
            Err(e) => {
                return Err(AppError::FFmpegExecutionError {
                    reason: format!("Failed to check FFmpeg status: {}", e),
                });
            }
        }
    }
}

/// Execute FFmpeg with progress callback
///
/// Parses progress from stderr and calls the callback with each update.
/// The callback receives the current progress in milliseconds.
pub fn execute_with_progress<F>(
    command: &FFmpegCommand,
    mut progress_callback: F,
) -> AppResult<FFmpegOutput>
where
    F: FnMut(FFmpegProgress) + Send + 'static,
{
    let args = command.build_args()?;
    let executable = command.executable().to_owned();

    info!("Executing FFmpeg with progress tracking: {}", args.join(" "));
    let start = Instant::now();

    let mut child = Command::new(executable.as_str())
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AppError::FFmpegNotFound
            } else {
                AppError::FFmpegExecutionError {
                    reason: format!("Failed to spawn FFmpeg: {}", e),
                }
            }
        })?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::FFmpegExecutionError {
            reason: "Failed to capture stdout".to_string(),
        })?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::FFmpegExecutionError {
            reason: "Failed to capture stderr".to_string(),
        })?;

    // Read stdout in a separate thread
    let stdout_handle = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        let mut output = String::new();
        for line in reader.lines().map_while(Result::ok) {
            output.push_str(&line);
            output.push('\n');
        }
        output
    });

    // Read stderr and parse progress
    let stderr_handle = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        let mut all_output = String::new();
        let mut line_buffer = String::new();

        // FFmpeg writes progress as carriage-return delimited lines
        for byte in reader.bytes().map_while(Result::ok) {
            let ch = byte as char;
            if ch == '\r' || ch == '\n' {
                if !line_buffer.is_empty() {
                    // Try to parse progress from this line
                    if let Some(progress) = parse_ffmpeg_progress(&line_buffer) {
                        progress_callback(progress);
                    }
                    all_output.push_str(&line_buffer);
                    all_output.push('\n');
                    line_buffer.clear();
                }
            } else {
                line_buffer.push(ch);
            }
        }

        // Handle any remaining content
        if !line_buffer.is_empty() {
            if let Some(progress) = parse_ffmpeg_progress(&line_buffer) {
                progress_callback(progress);
            }
            all_output.push_str(&line_buffer);
        }
        all_output
    });

    // Wait for the process to complete
    let status = child.wait().map_err(|e| AppError::FFmpegExecutionError {
        reason: format!("Failed to wait for FFmpeg: {}", e),
    })?;

    let stdout_content = stdout_handle.join().unwrap_or_default();
    let stderr_content = stderr_handle.join().unwrap_or_default();
    let duration = start.elapsed();

    let exit_code = status.code().unwrap_or(-1);

    if exit_code != 0 {
        error!(
            "FFmpeg failed with exit code {}: {}",
            exit_code, stderr_content
        );
    } else {
        info!("FFmpeg completed successfully in {:?}", duration);
    }

    Ok(FFmpegOutput {
        exit_code,
        stdout: stdout_content,
        stderr: stderr_content,
        duration,
    })
}

/// Parse FFmpeg progress from stderr output line
///
/// FFmpeg outputs progress in format:
/// `frame=12345 fps=30 size=1234kB time=00:15:30.45 bitrate=128kbits/s speed=2.5x`
///
/// Or when using `-progress pipe:1`, each field is on its own line:
/// `out_time_ms=123456000` (note: microseconds, not milliseconds!)
/// `out_time=00:00:05.12`
/// `speed=2.5x`
pub fn parse_ffmpeg_progress(line: &str) -> Option<FFmpegProgress> {
    let mut progress = FFmpegProgress::default();
    let line = line.trim();

    // Progress pipe format: key=value lines (out_time_ms, out_time, speed, progress)
    // Each field comes on its own line when using -progress pipe:1
    // Handle each possible key separately
    if line.starts_with("out_time_us=") || line.starts_with("out_time_ms=") {
        // out_time_us and out_time_ms are in MICROSECONDS despite the name!
        // FFmpeg documentation: "out_time_us: time in microseconds"
        let parts: Vec<&str> = line.splitn(2, '=').collect();
        if parts.len() == 2 {
            if let Ok(us) = parts[1].trim().parse::<u64>() {
                // Convert microseconds to milliseconds
                progress.time_ms = us / 1000;
                return if progress.is_valid() { Some(progress) } else { None };
            }
        }
    }
    
    if line.starts_with("out_time=") {
        let parts: Vec<&str> = line.splitn(2, '=').collect();
        if parts.len() == 2 {
            if let Some(ms) = parse_time_to_ms(parts[1].trim()) {
                progress.time_ms = ms;
                return if progress.is_valid() { Some(progress) } else { None };
            }
        }
    }
    
    // Speed line from progress pipe
    if line.starts_with("speed=") {
        let parts: Vec<&str> = line.splitn(2, '=').collect();
        if parts.len() == 2 {
            let s = parts[1].trim().trim_end_matches('x');
            if s != "N/A" {
                if let Ok(speed) = s.parse::<f32>() {
                    progress.speed = Some(speed);
                    // Speed alone isn't enough to report progress
                }
            }
        }
        return None; // Speed line doesn't provide time info
    }

    // Classic stderr progress line with time=
    if !line.contains("time=") {
        return None;
    }

    // Parse time=HH:MM:SS.ms
    if let Some(time_pos) = line.find("time=") {
        let time_str = &line[time_pos + 5..];
        let time_str = time_str.split_whitespace().next().unwrap_or("");
        if let Some(ms) = parse_time_to_ms(time_str) {
            progress.time_ms = ms;
        }
    }

    // Parse frame=N
    if let Some(frame_pos) = line.find("frame=") {
        let frame_str = &line[frame_pos + 6..];
        let frame_str = frame_str.split_whitespace().next().unwrap_or("");
        if let Ok(frame) = frame_str.trim().parse::<u64>() {
            progress.frame = Some(frame);
        }
    }

    // Parse fps=N
    if let Some(fps_pos) = line.find("fps=") {
        let fps_str = &line[fps_pos + 4..];
        let fps_str = fps_str.split_whitespace().next().unwrap_or("");
        if let Ok(fps) = fps_str.trim().parse::<f32>() {
            progress.fps = Some(fps);
        }
    }

    // Parse size=NkB
    if let Some(size_pos) = line.find("size=") {
        let size_str = &line[size_pos + 5..];
        let size_str = size_str.split_whitespace().next().unwrap_or("");
        progress.total_size = parse_size_to_bytes(size_str);
    }

    // Parse bitrate=Nkbits/s
    if let Some(br_pos) = line.find("bitrate=") {
        let br_str = &line[br_pos + 8..];
        let br_str = br_str.split_whitespace().next().unwrap_or("");
        if let Some(kbps) = parse_bitrate(br_str) {
            progress.bitrate_kbps = Some(kbps);
        }
    }

    // Parse speed=Nx
    if let Some(speed_pos) = line.find("speed=") {
        let speed_str = &line[speed_pos + 6..];
        let speed_str = speed_str.split_whitespace().next().unwrap_or("");
        let speed_str = speed_str.trim_end_matches('x');
        if let Ok(speed) = speed_str.parse::<f32>() {
            progress.speed = Some(speed);
        }
    }

    if progress.is_valid() {
        Some(progress)
    } else {
        None
    }
}

/// Parse time string (HH:MM:SS.ms) to milliseconds
fn parse_time_to_ms(time_str: &str) -> Option<u64> {
    // Handle format: HH:MM:SS.ms or MM:SS.ms
    let parts: Vec<&str> = time_str.split(':').collect();

    match parts.len() {
        3 => {
            // HH:MM:SS.ms
            let hours: u64 = parts[0].parse().ok()?;
            let minutes: u64 = parts[1].parse().ok()?;
            let seconds_parts: Vec<&str> = parts[2].split('.').collect();
            
            let seconds: u64 = seconds_parts[0].parse().ok()?;
            let millis: u64 = if seconds_parts.len() > 1 {
                let ms_str = seconds_parts[1];
                // Pad or truncate to 3 digits
                let ms_str = if ms_str.len() > 3 { &ms_str[..3] } else { ms_str };
                let padding = 3 - ms_str.len();
                let ms_val: u64 = ms_str.parse().ok()?;
                ms_val * 10u64.pow(padding as u32)
            } else {
                0
            };

            Some(hours * 3600 * 1000 + minutes * 60 * 1000 + seconds * 1000 + millis)
        }
        2 => {
            // MM:SS.ms
            let minutes: u64 = parts[0].parse().ok()?;
            let seconds_parts: Vec<&str> = parts[1].split('.').collect();
            
            let seconds: u64 = seconds_parts[0].parse().ok()?;
            let millis: u64 = if seconds_parts.len() > 1 {
                let ms_str = seconds_parts[1];
                let ms_str = if ms_str.len() > 3 { &ms_str[..3] } else { ms_str };
                let padding = 3 - ms_str.len();
                ms_str.parse::<u64>().ok()? * 10u64.pow(padding as u32)
            } else {
                0
            };

            Some(minutes * 60 * 1000 + seconds * 1000 + millis)
        }
        _ => None,
    }
}

/// Parse size string (e.g., "1234kB") to bytes
fn parse_size_to_bytes(size_str: &str) -> Option<u64> {
    let size_str = size_str.to_lowercase();
    
    if let Some(kb_str) = size_str.strip_suffix("kb") {
        let kb: u64 = kb_str.parse().ok()?;
        Some(kb * 1024)
    } else if let Some(mb_str) = size_str.strip_suffix("mb") {
        let mb: u64 = mb_str.parse().ok()?;
        Some(mb * 1024 * 1024)
    } else if let Some(gb_str) = size_str.strip_suffix("gb") {
        let gb: u64 = gb_str.parse().ok()?;
        Some(gb * 1024 * 1024 * 1024)
    } else if let Some(b_str) = size_str.strip_suffix("b") {
        b_str.parse().ok()
    } else {
        size_str.parse().ok()
    }
}

/// Parse bitrate string (e.g., "128kbits/s") to kbps
fn parse_bitrate(br_str: &str) -> Option<f32> {
    let br_str = br_str.to_lowercase();
    
    if let Some(kbits_str) = br_str.strip_suffix("kbits/s") {
        kbits_str.parse().ok()
    } else if let Some(kbps_str) = br_str.strip_suffix("kbps") {
        kbps_str.parse().ok()
    } else if let Some(mbits_str) = br_str.strip_suffix("mbits/s") {
        let mbits: f32 = mbits_str.parse().ok()?;
        Some(mbits * 1000.0)
    } else {
        // Try parsing as plain number (assumed to be kbps)
        br_str.parse().ok()
    }
}

/// Calculate progress percentage
///
/// Returns a value between 0.0 and 100.0
pub fn calculate_progress_percent(current_ms: u64, total_ms: u64) -> f32 {
    if total_ms == 0 {
        return 0.0;
    }
    
    let percent = (current_ms as f64 / total_ms as f64) * 100.0;
    percent.clamp(0.0, 100.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ffmpeg_progress_valid() {
        let line = "frame=12345 fps=30.0 size=1234kB time=00:15:30.45 bitrate=128kbits/s speed=2.5x";
        let progress = parse_ffmpeg_progress(line).expect("Should parse progress");

        assert_eq!(progress.frame, Some(12345));
        assert_eq!(progress.fps, Some(30.0));
        assert_eq!(progress.total_size, Some(1234 * 1024));
        assert_eq!(progress.time_ms, (15 * 60 + 30) * 1000 + 450);
        assert_eq!(progress.bitrate_kbps, Some(128.0));
        assert_eq!(progress.speed, Some(2.5));
    }

    #[test]
    fn test_parse_ffmpeg_progress_audio_only() {
        let line = "size=5432kB time=01:23:45.67 bitrate=128kbits/s speed=3.2x";
        let progress = parse_ffmpeg_progress(line).expect("Should parse progress");

        assert_eq!(progress.frame, None);
        assert_eq!(progress.fps, None);
        assert_eq!(progress.time_ms, (1 * 3600 + 23 * 60 + 45) * 1000 + 670);
        assert_eq!(progress.bitrate_kbps, Some(128.0));
        assert_eq!(progress.speed, Some(3.2));
    }

    #[test]
    fn test_parse_ffmpeg_progress_invalid() {
        let line = "Opening 'input.mp3' for reading";
        assert!(parse_ffmpeg_progress(line).is_none());
    }

    #[test]
    fn test_parse_time_to_ms() {
        assert_eq!(parse_time_to_ms("00:00:00.00"), Some(0));
        assert_eq!(parse_time_to_ms("00:00:01.00"), Some(1000));
        assert_eq!(parse_time_to_ms("00:01:00.00"), Some(60000));
        assert_eq!(parse_time_to_ms("01:00:00.00"), Some(3600000));
        assert_eq!(parse_time_to_ms("01:23:45.67"), Some((1 * 3600 + 23 * 60 + 45) * 1000 + 670));
        assert_eq!(parse_time_to_ms("00:15:30.45"), Some((15 * 60 + 30) * 1000 + 450));
    }

    #[test]
    fn test_parse_size_to_bytes() {
        assert_eq!(parse_size_to_bytes("1024kB"), Some(1024 * 1024));
        assert_eq!(parse_size_to_bytes("1MB"), Some(1024 * 1024));
        assert_eq!(parse_size_to_bytes("100b"), Some(100));
    }

    #[test]
    fn test_parse_bitrate() {
        assert_eq!(parse_bitrate("128kbits/s"), Some(128.0));
        assert_eq!(parse_bitrate("256kbps"), Some(256.0));
        assert_eq!(parse_bitrate("1.5mbits/s"), Some(1500.0));
    }

    #[test]
    fn test_calculate_progress_percent() {
        assert_eq!(calculate_progress_percent(0, 1000), 0.0);
        assert_eq!(calculate_progress_percent(500, 1000), 50.0);
        assert_eq!(calculate_progress_percent(1000, 1000), 100.0);
        assert_eq!(calculate_progress_percent(1500, 1000), 100.0); // Clamped
        assert_eq!(calculate_progress_percent(100, 0), 0.0); // Zero total
    }

    #[test]
    fn test_ffmpeg_output_success() {
        let output = FFmpegOutput {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            duration: Duration::from_secs(1),
        };
        assert!(output.success());

        let output = FFmpegOutput {
            exit_code: 1,
            stdout: String::new(),
            stderr: "Error".to_string(),
            duration: Duration::from_secs(1),
        };
        assert!(!output.success());
    }

    #[test]
    fn test_progress_accumulator_pipe_format() {
        let mut acc = FFmpegProgressAccumulator::new();

        // Simulate FFmpeg -progress pipe:1 output (each field on its own line)
        assert!(acc.parse_line("out_time_us=5000000").is_none());
        assert!(acc.parse_line("bitrate=128.5kbits/s").is_none());
        assert!(acc.parse_line("speed=2.5x").is_none());

        // On progress= line, we get the accumulated result
        let progress = acc.parse_line("progress=continue").expect("Should emit progress");
        assert_eq!(progress.time_ms, 5000); // 5000000 microseconds = 5000 ms
        assert_eq!(progress.speed, Some(2.5));
        assert_eq!(progress.bitrate_kbps, Some(128.5));
    }

    #[test]
    fn test_progress_accumulator_out_time_format() {
        let mut acc = FFmpegProgressAccumulator::new();

        // Test out_time= format (HH:MM:SS.ms)
        assert!(acc.parse_line("out_time=00:01:30.500").is_none());
        assert!(acc.parse_line("speed=3.0x").is_none());

        let progress = acc.parse_line("progress=continue").expect("Should emit progress");
        assert_eq!(progress.time_ms, 90500); // 1 min 30.5 sec = 90500 ms
        assert_eq!(progress.speed, Some(3.0));
    }

    #[test]
    fn test_progress_accumulator_multiple_blocks() {
        let mut acc = FFmpegProgressAccumulator::new();

        // First block
        acc.parse_line("out_time_us=1000000");
        acc.parse_line("speed=1.5x");
        let p1 = acc.parse_line("progress=continue").unwrap();
        assert_eq!(p1.time_ms, 1000);
        assert_eq!(p1.speed, Some(1.5));

        // Second block - time should update, speed should update
        acc.parse_line("out_time_us=2000000");
        acc.parse_line("speed=2.0x");
        let p2 = acc.parse_line("progress=continue").unwrap();
        assert_eq!(p2.time_ms, 2000);
        assert_eq!(p2.speed, Some(2.0));
    }
}
