use anyhow::{Context, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Sample, SampleFormat, Stream, StreamConfig,
};
use parakeet_rs::Nemotron;
use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc, Mutex,
};

// Constants
const MODEL_DIR: &str = "nemotron-speech-streaming-en-0.6b";
const TARGET_HZ: u32 = 16_000;
const CHUNK_SAMPLES: usize = 8_960;
const NUM_BARS: usize = 10;

type Levels = Arc<Mutex<Vec<f32>>>;

pub struct Transcriber {
    model: Nemotron,
    _stream: Stream,
    audio_rx: Receiver<Vec<f32>>,
    buffer: Vec<f32>,
    levels: Levels,
}

impl Transcriber {
    pub fn new() -> Result<Self> {
        let model = Nemotron::from_pretrained(MODEL_DIR, None)?;
        let levels = Arc::new(Mutex::new(vec![0.0; NUM_BARS]));
        let (stream, receiver) = open_input_stream(levels.clone())?;
        stream.play()?;
        Ok(Self {
            model,
            _stream: stream,
            audio_rx: receiver,
            buffer: Vec::with_capacity(CHUNK_SAMPLES * 4),
            levels,
        })
    }

    pub fn poll(&mut self) -> Result<String> {
        // Drain all pending audio
        let mut output = String::new();
        self.drain_audio();

        // Transcribe complete chunks
        while self.buffer.len() >= CHUNK_SAMPLES {
            let chunk = self.buffer.drain(..CHUNK_SAMPLES).collect::<Vec<_>>();
            let text = self.model.transcribe_chunk(&chunk)?;
            if !text.is_empty() {
                output.push_str(&text);
            }
        }
        Ok(output)
    }

    pub fn has_pending(&self) -> bool {
        !self.buffer.is_empty() || self.audio_rx.try_recv().is_ok()
    }

    pub fn levels(&self) -> Vec<f32> {
        self.levels.lock().map(|l| l.clone()).unwrap_or_default()
    }

    fn drain_audio(&mut self) {
        while let Ok(samples) = self.audio_rx.try_recv() {
            self.buffer.extend_from_slice(&samples);
        }
    }

    pub fn drain(&mut self) {
        self.drain_audio();
        self.buffer.clear();
        self.model.reset();
    }
}

fn open_input_stream(levels: Levels) -> Result<(Stream, Receiver<Vec<f32>>)> {
    let device = cpal::default_host()
        .default_input_device()
        .context("No input device found")?;

    // Pick config that supports 16 kHz, fewest channels, prefer f32
    let supported = device
        .supported_input_configs()?
        .filter(|c| c.min_sample_rate() <= TARGET_HZ && c.max_sample_rate() >= TARGET_HZ)
        .min_by_key(|c| {
            (
                c.channels() as u32,
                (c.sample_format() != SampleFormat::F32) as u32,
            )
        })
        .context("Device does not support {TARGET_HZ} Hz")?
        .with_sample_rate(TARGET_HZ);

    let channels = supported.channels() as usize;
    let format = supported.sample_format();
    let config: StreamConfig = supported.into();
    let (sender, receiver) = mpsc::channel();
    let stream = match format {
        SampleFormat::F32 => build_stream::<f32>(&device, &config, channels, sender, levels)?,
        SampleFormat::I16 => build_stream::<i16>(&device, &config, channels, sender, levels)?,
        SampleFormat::U16 => build_stream::<u16>(&device, &config, channels, sender, levels)?,
        other => anyhow::bail!("Unsupported sample format: {other:?}"),
    };
    Ok((stream, receiver))
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    channels: usize,
    sender: Sender<Vec<f32>>,
    levels: Levels,
) -> Result<Stream>
where
    T: Sample + cpal::SizedSample + 'static,
    f32: cpal::FromSample<T>,
{
    Ok(device.build_input_stream(
        config,
        move |data: &[T], _| {
            // Average channels down to mono
            let mono: Vec<f32> = if channels <= 1 {
                data.iter().map(|&s| s.to_sample::<f32>()).collect()
            } else {
                data.chunks_exact(channels)
                    .map(|frame| {
                        frame.iter().map(|&s| s.to_sample::<f32>()).sum::<f32>() / channels as f32
                    })
                    .collect()
            };

            // Compute per-bar RMS levels for waveform
            if let Ok(mut lvls) = levels.lock() {
                let bar_size = (mono.len() / NUM_BARS).max(1);
                for i in 0..NUM_BARS {
                    let start = i * bar_size;
                    let end = (start + bar_size).min(mono.len());
                    if start < mono.len() {
                        let rms = (mono[start..end].iter().map(|&s| s * s).sum::<f32>()
                            / (end - start) as f32)
                            .sqrt();
                        lvls[i] = lvls[i] * 0.3 + rms * 0.7;
                    }
                }
            }

            let _ = sender.send(mono);
        },
        |err| eprintln!("Audio error: {err}"),
        None,
    )?)
}
