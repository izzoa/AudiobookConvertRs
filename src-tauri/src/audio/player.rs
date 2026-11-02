//! Audio playback service using rodio
//!
//! Provides audio preview functionality for files and chapters.
//! Uses a dedicated thread for audio playback since rodio types are not Send.

use crate::error::{AppError, AppResult};
use camino::Utf8Path;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tracing::{debug, error, info};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackStatus {
    pub state: PlaybackState,
    pub current_path: Option<String>,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f32,
}

impl Default for PlaybackStatus {
    fn default() -> Self {
        Self {
            state: PlaybackState::Stopped,
            current_path: None,
            position_ms: 0,
            duration_ms: 0,
            volume: 1.0,
        }
    }
}

enum PlayerCommand {
    Play { path: String, start_ms: u64 },
    Pause,
    Resume,
    Stop,
    SetVolume(f32),
    GetStatus(Sender<PlaybackStatus>),
    Shutdown,
}

struct PlayerState {
    current_path: Option<String>,
    duration_ms: u64,
    start_position_ms: u64,
    volume: f32,
    is_playing: bool,
    is_paused: bool,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            current_path: None,
            duration_ms: 0,
            start_position_ms: 0,
            volume: 1.0,
            is_playing: false,
            is_paused: false,
        }
    }
}

fn run_player_thread(rx: Receiver<PlayerCommand>) {
    use rodio::{Decoder, OutputStream, Sink, Source};
    use std::fs::File;
    use std::io::BufReader;

    let mut state = PlayerState::default();
    let mut current_sink: Option<Sink> = None;
    let mut current_stream: Option<OutputStream> = None;

    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(cmd) => match cmd {
                PlayerCommand::Play { path, start_ms } => {
                    if let Some(sink) = current_sink.take() {
                        sink.stop();
                    }
                    current_stream = None;

                    match File::open(&path) {
                        Ok(file) => {
                            let reader = BufReader::new(file);
                            match Decoder::new(reader) {
                                Ok(source) => {
                                    let duration_ms = source
                                        .total_duration()
                                        .map(|d| d.as_millis() as u64)
                                        .unwrap_or(0);

                                    match OutputStream::try_default() {
                                        Ok((stream, stream_handle)) => {
                                            match Sink::try_new(&stream_handle) {
                                                Ok(sink) => {
                                                    if start_ms > 0 {
                                                        let skip_source = source.skip_duration(
                                                            Duration::from_millis(start_ms),
                                                        );
                                                        sink.append(skip_source);
                                                    } else {
                                                        sink.append(source);
                                                    }
                                                    sink.set_volume(state.volume);

                                                    state.current_path = Some(path.clone());
                                                    state.duration_ms = duration_ms;
                                                    state.start_position_ms = start_ms;
                                                    state.is_playing = true;
                                                    state.is_paused = false;

                                                    current_sink = Some(sink);
                                                    current_stream = Some(stream);
                                                    info!("Started playback: {}", path);
                                                }
                                                Err(e) => error!("Failed to create sink: {}", e),
                                            }
                                        }
                                        Err(e) => error!("Failed to get audio output: {}", e),
                                    }
                                }
                                Err(e) => error!("Failed to decode audio: {}", e),
                            }
                        }
                        Err(e) => error!("Failed to open file: {}", e),
                    }
                }
                PlayerCommand::Pause => {
                    if let Some(ref sink) = current_sink {
                        sink.pause();
                        state.is_paused = true;
                        debug!("Paused playback");
                    }
                }
                PlayerCommand::Resume => {
                    if let Some(ref sink) = current_sink {
                        sink.play();
                        state.is_paused = false;
                        debug!("Resumed playback");
                    }
                }
                PlayerCommand::Stop => {
                    if let Some(sink) = current_sink.take() {
                        sink.stop();
                    }
                    current_stream = None;
                    state.current_path = None;
                    state.is_playing = false;
                    state.is_paused = false;
                    debug!("Stopped playback");
                }
                PlayerCommand::SetVolume(vol) => {
                    state.volume = vol.clamp(0.0, 1.0);
                    if let Some(ref sink) = current_sink {
                        sink.set_volume(state.volume);
                    }
                    debug!("Set volume to {}", state.volume);
                }
                PlayerCommand::GetStatus(tx) => {
                    let is_empty = current_sink.as_ref().map(|s| s.empty()).unwrap_or(true);
                    if is_empty && state.is_playing {
                        state.is_playing = false;
                        state.is_paused = false;
                    }

                    let status = PlaybackStatus {
                        state: if !state.is_playing {
                            PlaybackState::Stopped
                        } else if state.is_paused {
                            PlaybackState::Paused
                        } else {
                            PlaybackState::Playing
                        },
                        current_path: state.current_path.clone(),
                        position_ms: state.start_position_ms,
                        duration_ms: state.duration_ms,
                        volume: state.volume,
                    };
                    let _ = tx.send(status);
                }
                PlayerCommand::Shutdown => {
                    if let Some(sink) = current_sink.take() {
                        sink.stop();
                    }
                    break;
                }
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Some(ref sink) = current_sink {
                    if sink.empty() && state.is_playing {
                        state.is_playing = false;
                        state.is_paused = false;
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    debug!("Player thread shutting down");
}

pub struct AudioPlayer {
    tx: Mutex<Option<Sender<PlayerCommand>>>,
    _thread: Mutex<Option<JoinHandle<()>>>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            run_player_thread(rx);
        });

        Self {
            tx: Mutex::new(Some(tx)),
            _thread: Mutex::new(Some(handle)),
        }
    }

    fn send_command(&self, cmd: PlayerCommand) -> AppResult<()> {
        let guard = self.tx.lock();
        if let Some(ref tx) = *guard {
            tx.send(cmd)
                .map_err(|_| AppError::AudioPlayback("Player thread not running".to_string()))?;
        }
        Ok(())
    }

    pub fn play(&self, path: &Utf8Path, start_ms: Option<u64>) -> AppResult<()> {
        debug!("Playing audio: {} from {}ms", path, start_ms.unwrap_or(0));
        if !path.exists() {
            return Err(AppError::FileNotFound(path.as_std_path().to_path_buf()));
        }
        self.send_command(PlayerCommand::Play {
            path: path.to_string(),
            start_ms: start_ms.unwrap_or(0),
        })
    }

    pub fn pause(&self) -> AppResult<()> {
        self.send_command(PlayerCommand::Pause)
    }

    pub fn resume(&self) -> AppResult<()> {
        self.send_command(PlayerCommand::Resume)
    }

    pub fn stop(&self) -> AppResult<()> {
        self.send_command(PlayerCommand::Stop)
    }

    pub fn seek(&self, position_ms: u64) -> AppResult<()> {
        let status = self.get_status();
        if let Some(path) = status.current_path {
            self.send_command(PlayerCommand::Play {
                path,
                start_ms: position_ms,
            })
        } else {
            Err(AppError::AudioPlayback(
                "No audio is currently playing".to_string(),
            ))
        }
    }

    pub fn set_volume(&self, volume: f32) -> AppResult<()> {
        self.send_command(PlayerCommand::SetVolume(volume))
    }

    pub fn get_status(&self) -> PlaybackStatus {
        let (tx, rx) = mpsc::channel();
        let _ = self.send_command(PlayerCommand::GetStatus(tx));
        rx.recv_timeout(Duration::from_millis(500))
            .unwrap_or_default()
    }

    pub fn is_playing(&self) -> bool {
        self.get_status().state == PlaybackState::Playing
    }
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        let _ = self.send_command(PlayerCommand::Shutdown);
    }
}

pub type SharedPlayer = Arc<AudioPlayer>;

pub fn create_shared_player() -> SharedPlayer {
    Arc::new(AudioPlayer::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playback_status_default() {
        let status = PlaybackStatus::default();
        assert_eq!(status.state, PlaybackState::Stopped);
        assert!(status.current_path.is_none());
        assert_eq!(status.position_ms, 0);
        assert_eq!(status.volume, 1.0);
    }

    #[test]
    fn test_player_creation() {
        let player = AudioPlayer::new();
        let status = player.get_status();
        assert_eq!(status.state, PlaybackState::Stopped);
    }

    #[test]
    fn test_set_volume() {
        let player = AudioPlayer::new();
        player.set_volume(0.5).unwrap();
        let status = player.get_status();
        assert!((status.volume - 0.5).abs() < 0.01);
    }
}
