//! Runtime errors.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    /// PCM geometry invalid (channels/rate/length).
    InvalidBuffer,
    /// Unknown buffer or voice handle.
    InvalidHandle,
    /// Voice pool exhausted.
    NoFreeVoice,
    /// Event path not registered.
    EventNotFound,
    /// Audio device / backend failure.
    Device(String),
    /// Feature not available in this build.
    BackendUnavailable,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::InvalidBuffer => f.write_str("invalid PCM buffer"),
            RuntimeError::InvalidHandle => f.write_str("invalid handle"),
            RuntimeError::NoFreeVoice => f.write_str("no free voice"),
            RuntimeError::EventNotFound => f.write_str("event not found"),
            RuntimeError::Device(msg) => write!(f, "audio device: {msg}"),
            RuntimeError::BackendUnavailable => {
                f.write_str("audio backend unavailable (enable feature backend-cpal)")
            }
        }
    }
}

impl std::error::Error for RuntimeError {}
