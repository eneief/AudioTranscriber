use anyhow::{Result, anyhow};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use crossbeam_channel::{Receiver, Sender, bounded};

use std::time::Duration;

// List input devices names
pub fn list_devices() -> Result<Vec<String>> {
    let host = cpal::default_host();
    let mut names = Vec::new();
    for dev in host.input_devices()? {
        names.push(dev.name()?);
    }
    if names.is_empty() {
        Err(anyhow!("no input devices found"))
    } else {
        Ok(names)
    }
}

// Configuration settings for recorder
pub struct RecorderConfig {
    /// None = default or select from list_devices() list
    pub device_index: Option<usize>,
    /// Downmix the audio channels to mono
    pub prefer_mono: bool,
    /// Capacity in # of chunks for inter-thread channel
    pub queue_chunks_capacity: usize,
}

impl Default for RecorderConfig {
    fn default() -> Self {
        Self {
            device_index: None,
            prefer_mono: true,
            queue_chunks_capacity: 32,
        }
    }
}

pub struct Recorder {
    device_name: String,
    input_channels: u16,
    sample_rate: u32,
    prefer_mono: bool,
    stream: Option<Stream>,
    rx: Receiver<Vec<i16>>,
    _tx: Sender<Vec<i16>>,
}

impl Recorder {
    pub fn recv_chunk_timeout(&self, dur: Duration) -> Option<Vec<i16>> {
        self.rx.recv_timeout(dur).ok()
    }

    pub fn try_recv_chunk(&self) -> Option<Vec<i16>> {
        self.rx.try_recv().ok()
    }

    // Open device and build CPAL input stream
    pub fn open(cfg: RecorderConfig) -> Result<Self> {
        let host = cpal::default_host();

        let device = if let Some(idx) = cfg.device_index {
            let mut it = host.input_devices()?;
            it.nth(idx)
                .ok_or_else(|| anyhow!("invalid device index {}", idx))?
        } else {
            host.default_input_device()
                .ok_or_else(|| anyhow!("no default input device"))?
        };
        let device_name = device.name().unwrap_or_else(|_| "<unknown>".into());

        // device input config used
        let supported = device
            .default_input_config()
            .map_err(|e| anyhow!("default_input_config: {e}"))?;
        let sample_format = supported.sample_format();
        // let mut config: StreamConfig = supported.config().clone();
        let config: StreamConfig = supported.config().clone();
        let input_channels = config.channels;
        let sample_rate = config.sample_rate.0;

        let (tx, rx) = bounded::<Vec<i16>>(cfg.queue_chunks_capacity);

        let prefer_mono = cfg.prefer_mono;
        let stream = match sample_format {
            SampleFormat::F32 => {
                build_stream_f32(&device, &config, input_channels, prefer_mono, tx.clone())?
            }
            SampleFormat::I16 => {
                build_stream_i16(&device, &config, input_channels, prefer_mono, tx.clone())?
            }
            SampleFormat::U16 => {
                build_stream_u16(&device, &config, input_channels, prefer_mono, tx.clone())?
            }
            _ => return Err(anyhow!("unsupported sample format: {:?}", sample_format)),
        };

        Ok(Self {
            device_name,
            input_channels,
            sample_rate,
            prefer_mono,
            stream: Some(stream),
            rx,
            _tx: tx,
        })
    }

    // Start/Stop + getters
    pub fn start(&mut self) -> Result<()> {
        if let Some(s) = &self.stream {
            s.play()?;
            Ok(())
        } else {
            Err(anyhow!("stream not initialized"))
        }
    }

    pub fn stop(&mut self) {
        self.stream.take();
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }
    pub fn input_channels(&self) -> u16 {
        self.input_channels
    }
    pub fn output_channels(&self) -> u16 {
        if self.prefer_mono {
            1
        } else {
            self.input_channels
        }
    }
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

// Per-format stream builders

fn build_stream_f32(
    device: &cpal::Device,
    config: &StreamConfig,
    input_channels: u16,
    prefer_mono: bool,
    tx: Sender<Vec<i16>>,
) -> Result<Stream> {
    let chans = input_channels as usize;
    let err_fn = |err| eprintln!("stream error: {err}");

    let stream = device.build_input_stream(
        config,
        move |data: &[f32], _info: &cpal::InputCallbackInfo| {
            if data.is_empty() {
                return;
            }
            let mut out = Vec::<i16>::with_capacity(if prefer_mono {
                data.len() / chans
            } else {
                data.len()
            });

            if prefer_mono && chans > 1 {
                for frame in data.chunks_exact(chans) {
                    let mut acc: i32 = 0;
                    for &s in frame {
                        acc += f32_to_i16(s) as i32;
                    }
                    out.push((acc / chans as i32) as i16);
                }
            } else {
                for &s in data {
                    out.push(f32_to_i16(s));
                }
            }

            let _ = tx.try_send(out);
        },
        err_fn,
        None,
    )?;
    Ok(stream)
}

fn build_stream_i16(
    device: &cpal::Device,
    config: &StreamConfig,
    input_channels: u16,
    prefer_mono: bool,
    tx: Sender<Vec<i16>>,
) -> Result<Stream> {
    let chans = input_channels as usize;
    let err_fn = |err| eprintln!("stream error: {err}");

    let stream = device.build_input_stream(
        config,
        move |data: &[i16], _info: &cpal::InputCallbackInfo| {
            if data.is_empty() {
                return;
            }
            let mut out = Vec::<i16>::with_capacity(if prefer_mono {
                data.len() / chans
            } else {
                data.len()
            });

            if prefer_mono && chans > 1 {
                for frame in data.chunks_exact(chans) {
                    let mut acc: i32 = 0;
                    for &s in frame {
                        acc += s as i32;
                    }
                    out.push((acc / chans as i32) as i16);
                }
            } else {
                out.extend_from_slice(data);
            }

            let _ = tx.try_send(out);
        },
        err_fn,
        None,
    )?;
    Ok(stream)
}

fn build_stream_u16(
    device: &cpal::Device,
    config: &StreamConfig,
    input_channels: u16,
    prefer_mono: bool,
    tx: Sender<Vec<i16>>,
) -> Result<Stream> {
    let chans = input_channels as usize;
    let err_fn = |err| eprintln!("stream error: {err}");

    let stream = device.build_input_stream(
        config,
        move |data: &[u16], _info: &cpal::InputCallbackInfo| {
            if data.is_empty() {
                return;
            }
            let mut out = Vec::<i16>::with_capacity(if prefer_mono {
                data.len() / chans
            } else {
                data.len()
            });

            if prefer_mono && chans > 1 {
                for frame in data.chunks_exact(chans) {
                    let mut acc: i32 = 0;
                    for &s in frame {
                        acc += u16_to_i16(s) as i32;
                    }
                    out.push((acc / chans as i32) as i16);
                }
            } else {
                for &s in data {
                    out.push(u16_to_i16(s));
                }
            }

            let _ = tx.try_send(out);
        },
        err_fn,
        None,
    )?;
    Ok(stream)
}

// conversion functions to i16 for wav

#[inline]
fn f32_to_i16(s: f32) -> i16 {
    let clamped = s.clamp(-1.0, 1.0);
    (clamped * 32767.0).round() as i16
}

#[inline]
fn u16_to_i16(s: u16) -> i16 {
    (s as i32 - 32768) as i16
}
