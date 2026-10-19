//! Output backends.

use crate::error::RuntimeError;
use crate::RuntimeConfig;

/// Trait for device output (cpal or null).
pub trait OutputBackend: Send {
    fn sample_rate(&self) -> u32;
    fn channels(&self) -> u16;
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
}

/// Open the default output device, or a null backend if headless / open fails.
pub fn open_default(cfg: &RuntimeConfig) -> Result<Option<Box<dyn OutputBackend>>, RuntimeError> {
    #[cfg(feature = "backend-cpal")]
    {
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

    struct CpalBackend {
        sample_rate: u32,
        channels: u16,
        _stream: cpal::Stream,
    }

    impl OutputBackend for CpalBackend {
        fn sample_rate(&self) -> u32 {
            self.sample_rate
        }
        fn channels(&self) -> u16 {
            self.channels
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

        // S1: hold the device with a silent stream. A follow-up wires
        // Arc<Mutex<Runtime>> into the callback for true low-latency pull.
        let config = supported.config();
        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _| {
                    for s in data.iter_mut() {
                        *s = 0.0;
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
            _stream: stream,
        }))
    }
}
