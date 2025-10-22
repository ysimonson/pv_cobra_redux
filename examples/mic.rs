use std::cmp::min;
use std::{env, thread};
use std::sync::{Arc, Mutex, TryLockError};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::{command, Parser};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Sample;
use indicatif::ProgressBar;

struct AudioInputProcessor {
    resampler: samplerate::Samplerate,
    buf: Option<Vec<i16>>,
    cobra: pv_cobra_redux::Cobra,
    progress_bar: ProgressBar,
}

impl AudioInputProcessor {
    fn new(input_sample_rate: u32, frame_length: usize, channels: usize, access_key: String) -> Result<Self> {
        Ok(Self {
            resampler: samplerate::Samplerate::new(
                samplerate::ConverterType::SincBestQuality,
                input_sample_rate,
                pv_cobra_redux::sample_rate() as u32,
                channels,
            )?,
            buf: Some(Vec::with_capacity(frame_length)),
            cobra: pv_cobra_redux::Cobra::new(access_key)?,
            progress_bar: ProgressBar::new(100),
        })
    }
}

unsafe impl Send for AudioInputProcessor {}

fn convert_samples_to_f32<S: cpal::Sample>(data: &[S]) -> Vec<f32> {
    data.iter().map(|s| s.to_float_sample().to_sample()).collect()
}

#[derive(Parser)]
#[command(author, version, about, long_about = None, propagate_version = true)]
struct Cli {
    /// Name of the microphone device. If unspecified, the default device is
    /// used.
    #[arg(long)]
    mic_device_name: Option<String>,
}

struct Device {
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
}

impl Device {
    pub fn new(device: cpal::Device) -> Result<Self> {
        let config = device.default_input_config()?;
        Ok(Self { device, config })
    }

    fn new_from_maybe_device(device: Option<cpal::Device>) -> Result<Option<Self>> {
        match device {
            Some(device) => {
                let mic = Self::new(device)?;
                Ok(Some(mic))
            }
            None => Ok(None),
        }
    }

    pub fn new_from_default_device() -> Result<Option<Self>> {
        let host = cpal::default_host();
        let device = host.default_input_device();
        Self::new_from_maybe_device(device)
    }

    pub fn new_from_device_name<S: AsRef<str>>(name: S) -> Result<Option<Self>> {
        let host = cpal::default_host();
        let device = host
            .input_devices()?
            .find(|x| x.name().map(|y| y == name.as_ref()).unwrap_or(false));
        Self::new_from_maybe_device(device)
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let device = if let Some(mic_device_name) = cli.mic_device_name {
        Device::new_from_device_name(mic_device_name)
    } else {
        Device::new_from_default_device()
    };
    let device = device?.context("mic device not found")?;
    let access_key = env::var("PICOVOICE_ACCESS_KEY").context("missing environment variable `PICOVOICE_ACCESS_KEY`")?;
    let channels = device.config.channels();
    let frame_length = pv_cobra_redux::sample_rate() as usize;

    let proc = Arc::new(Mutex::new(AudioInputProcessor::new(
        device.config.sample_rate().0,
        frame_length,
        channels as usize,
        access_key
    )?));

    let add_samples = move |samples: &[f32]| -> Result<()> {
        match proc.try_lock() {
            Ok(mut guard) => {
                // Resample the stereo audio to the desired sample rate
                let resampled_stereo = guard.resampler.process(samples)?;

                let resampled_mono: Vec<i16> = if channels == 1 {
                    resampled_stereo
                        .iter()
                        .map(|s| (s * i16::MAX as f32).round() as i16)
                        .collect()
                } else {
                    // convert from stereo to mono
                    resampled_stereo
                        .chunks(2) // Iterate over pairs of samples (left, right)
                        .map(|chunk| {
                            let left = chunk[0];
                            let right = chunk[1];
                            let mono = (left + right) / 2.0; // Average the two channels
                            (mono * i16::MAX as f32).round() as i16
                        })
                        .collect()
                };

                let buf = guard.buf.as_mut().unwrap();
                buf.extend_from_slice(&resampled_mono);
                if buf.len() >= frame_length {
                    let mut buf = guard.buf.take().unwrap();
                    let confidence = guard.cobra.process(&buf)?;
                    buf.clear();
                    guard.buf = Some(buf);
                    guard.progress_bar.set_position((confidence * 100.0) as u64);
                }
            }
            Err(TryLockError::WouldBlock) => {
                eprintln!("microphone stream processing is falling behind");
            }
            Err(TryLockError::Poisoned(err)) => {
                bail!("microphone stream processing lock is poisoned: {err:?}");
            }
        }
        Ok(())
    };

    let handle_err = move |err: cpal::StreamError| {
        eprintln!("stream error: {err}");
    };

    let stream = match device.config.sample_format() {
        cpal::SampleFormat::I8 => device.device.build_input_stream(
            &device.config.clone().into(),
            move |data: &[i8], _: &_| {
                let samples = convert_samples_to_f32(data);
                add_samples(&samples).expect("failed to add samples");
            },
            handle_err,
            None,
        )?,
        cpal::SampleFormat::I16 => device.device.build_input_stream(
            &device.config.clone().into(),
            move |data: &[i16], _: &_| {
                let samples = convert_samples_to_f32(data);
                add_samples(&samples).expect("failed to add samples");
            },
            handle_err,
            None,
        )?,
        cpal::SampleFormat::I32 => device.device.build_input_stream(
            &device.config.clone().into(),
            move |data: &[i32], _: &_| {
                let samples = convert_samples_to_f32(data);
                add_samples(&samples).expect("failed to add samples");
            },
            handle_err,
            None,
        )?,
        cpal::SampleFormat::F32 => device.device.build_input_stream(
            &device.config.clone().into(),
            move |data: &[f32], _: &_| {
                add_samples(data).expect("failed to add samples");
            },
            handle_err,
            None,
        )?,
        sample_format => bail!("unsupported format: {sample_format}")
    };

    println!("VAD confidence:");
    stream.play()?;

    loop {
        thread::sleep(Duration::from_secs(1));
    }
}
