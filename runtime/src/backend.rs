//! Output backends — cpal with shared ring buffer for audible output.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::error::RuntimeError;
use crate::RuntimeConfig;

/// Trait for device output (cpal or null).
pub trait OutputBackend: Send {
    fn sample_rate(&self) -> u32;
    fn channels(&self) -> u16;
    /// Push interleaved samples into the device ring (no-op for null).
    fn push_samples(&self, interleaved: &[f32]);
}

/// Headless backend (tests / CI).
pub struct NullBackend {
    sample_rate: u32,
    channels: u16,
}

impl OutputBackend for NullBackend {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn channels(&self) -> u16 {
        self.channels
    }
    fn push_samples(&self, _interleaved: &[f32]) {}
}

/// Open the default output device, or a null backend if headless / open fails.
pub fn open_default(cfg: &RuntimeConfig) -> Result<Option<Box<dyn OutputBackend>>, RuntimeError> {
    #[cfg(feature = "backend-cpal")]
    {
        if !cfg.open_device {
            return Ok(Some(Box::new(NullBackend {
                sample_rate: cfg.sample_rate,
                channels: cfg.channels,
            })));
        }
        match cpal_backend::try_open(cfg) {
            Ok(b) => Ok(Some(b)),
            Err(e) => {
                eprintln!("ramod_runtime: cpal open failed ({e}); using null backend");
                Ok(Some(Box::new(NullBackend {
                    sample_rate: cfg.sample_rate,
                    channels: cfg.channels,
                })))
            }
        }
    }
    #[cfg(not(feature = "backend-cpal"))]
    {
        let _ = cfg.open_device;
        Ok(Some(Box::new(NullBackend {
            sample_rate: cfg.sample_rate,
            channels: cfg.channels,
        })))
    }
}

#[cfg(feature = "backend-cpal")]
mod cpal_backend {
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    const RING_CAP: usize = 48_000 * 2 * 2; // ~2s stereo @ 48k

    struct CpalBackend {
        sample_rate: u32,
        channels: u16,
        ring: Arc<Mutex<VecDeque<f32>>>,
        _stream: cpal::Stream,
    }

    // cpal::Stream is !Send on some platforms due to phantom; the stream is only
    // owned by this backend and never shared across threads after construction.
    unsafe impl Send for CpalBackend {}

    impl OutputBackend for CpalBackend {
        fn sample_rate(&self) -> u32 {
            self.sample_rate
        }
        fn channels(&self) -> u16 {
            self.channels
        }
        fn push_samples(&self, interleaved: &[f32]) {
            if let Ok(mut q) = self.ring.lock() {
                for &s in interleaved {
                    if q.len() >= RING_CAP {
                        // Drop oldest to avoid unbounded growth
                        let _ = q.pop_front();
                    }
                    q.push_back(s);
                }
            }
        }
    }

    pub fn try_open(cfg: &RuntimeConfig) -> Result<Box<dyn OutputBackend>, RuntimeError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| RuntimeError::Device("no default output device".into()))?;
        let supported = device
            .default_output_config()
            .map_err(|e| RuntimeError::Device(e.to_string()))?;

        let sample_rate = supported.sample_rate().0;
        let channels = supported.channels();
        let _ = cfg;

        let ring: Arc<Mutex<VecDeque<f32>>> =
            Arc::new(Mutex::new(VecDeque::with_capacity(RING_CAP / 4)));
        let ring_cb = Arc::clone(&ring);

        let config = supported.config();
        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _| {
                    if let Ok(mut q) = ring_cb.lock() {
                        for s in data.iter_mut() {
                            *s = q.pop_front().unwrap_or(0.0);
                        }
                    } else {
                        for s in data.iter_mut() {
                            *s = 0.0;
                        }
                    }
                },
                |err| eprintln!("ramod_runtime stream error: {err}"),
                None,
            )
            .map_err(|e| RuntimeError::Device(e.to_string()))?;
        stream
            .play()
            .map_err(|e| RuntimeError::Device(e.to_string()))?;

        Ok(Box::new(CpalBackend {
            sample_rate,
            channels,
            ring,
            _stream: stream,
        }))
    }
}
