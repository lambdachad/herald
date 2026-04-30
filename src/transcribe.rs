use crate::Result;
use cpal::{
    Sample, SampleFormat, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use parakeet_rs::Nemotron;
use std::sync::{
    Arc, Mutex,
    mpsc::{self, Receiver, Sender},
};

// Nemotron
const MODEL_DIR: &str = "nemotron-speech-streaming-en-0.6b-int4";
const TARGET_HZ: u32 = 16_000;
const CHUNK_SAMPLES: usize = 8_960; // 560ms at 16 kHz
const NUM_BARS: usize = 10;

/// Shared amplitude levels for waveform rendering
pub type Levels = Arc<Mutex<Vec<f32>>>;

pub struct Transcriber {
    model: Nemotron,
    stream: Option<Stream>,
    audio_rx: Option<Receiver<Vec<f32>>>,
    buffer: Vec<f32>,
    levels: Levels,
}

impl Transcriber {
    pub fn new() -> Result<Self> {
        let model = Nemotron::from_pretrained(MODEL_DIR, None)?;
        Ok(Self {
            model,
            stream: None,
            audio_rx: None,
            buffer: Vec::with_capacity(CHUNK_SAMPLES * 4),
            levels: Arc::new(Mutex::new(vec![0.0; NUM_BARS])),
        })
    }

    pub fn start_capture(&mut self) -> Result<()> {
        let (stream, receiver) = open_input_stream(self.levels.clone())?;
        stream.play()?;
        self.stream = Some(stream);
        self.audio_rx = Some(receiver);
        self.buffer.clear();
        Ok(())
    }

    pub fn stop_capture(&mut self) {
        self.stream = None;
        self.audio_rx = None;
        if let Ok(mut levels) = self.levels.lock() {
            levels.iter_mut().for_each(|v| *v = 0.0);
        }
    }

    pub fn poll(&mut self) -> Result<String> {
        // Drain all pending audio without blocking
        let mut output = String::new();
        if let Some(rx) = &self.audio_rx {
            loop {
                match rx.try_recv() {
                    Ok(samples) => self.buffer.extend_from_slice(&samples),
                    Err(_) => break,
                }
            }
        }

        // Every time we get a full chunk, transcribe it
        while self.buffer.len() >= CHUNK_SAMPLES {
            let chunk = self.buffer.drain(..CHUNK_SAMPLES).collect::<Vec<_>>();
            let text = self
                .model
                .transcribe_chunk(&chunk)
                .map_err(|e| format!("Transcription failed: {e}"))?;
            if !text.is_empty() {
                output.push_str(&text);
            }
        }
        Ok(output)
    }

    pub fn levels(&self) -> Levels {
        self.levels.clone()
    }

    pub fn is_recording(&self) -> bool {
        self.stream.is_some()
    }
}

fn open_input_stream(levels: Levels) -> Result<(Stream, Receiver<Vec<f32>>)> {
    let device = cpal::default_host()
        .default_input_device()
        .ok_or("No input device found")?;

    // Pick config that supports 16 kHz, fewest channels, prefer f32
    let supported = device
        .supported_input_configs()?
        .filter(|config| {
            config.min_sample_rate() <= TARGET_HZ && config.max_sample_rate() >= TARGET_HZ
        })
        .min_by_key(|config| {
            (
                config.channels() as u32,
                (config.sample_format() != SampleFormat::F32) as u32,
            )
        })
        .ok_or(format!("Device does not support {TARGET_HZ} Hz"))?
        .with_sample_rate(TARGET_HZ);

    // Build stream with config
    let channels = supported.channels() as usize;
    let format = supported.sample_format();
    let config: StreamConfig = supported.into();
    let (sender, receiver) = mpsc::channel();
    let stream = match format {
        SampleFormat::F32 => build_stream::<f32>(&device, &config, channels, sender, levels)?,
        SampleFormat::I16 => build_stream::<i16>(&device, &config, channels, sender, levels)?,
        SampleFormat::U16 => build_stream::<u16>(&device, &config, channels, sender, levels)?,
        other => return Err(format!("Unsupported sample format: {other:?}").into()),
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
            let mono = if channels <= 1 {
                data.iter()
                    .map(|&sample| sample.to_sample::<f32>())
                    .collect::<Vec<_>>()
            } else {
                // Average channels down to mono
                data.chunks_exact(channels)
                    .map(|frame| {
                        frame
                            .iter()
                            .map(|&sample| sample.to_sample::<f32>())
                            .sum::<f32>()
                            / channels as f32
                    })
                    .collect::<Vec<_>>()
            };

            // Compute per-bar RMS levels for waveform
            if !mono.is_empty() {
                let bar_size = (mono.len() / NUM_BARS).max(1);
                if let Ok(mut levels) = levels.lock() {
                    for i in 0..NUM_BARS {
                        let start = i * bar_size;
                        let end = (start + bar_size).min(mono.len());
                        if start < mono.len() {
                            let rms = (mono[start..end]
                                .iter()
                                .map(|&sample| sample * sample)
                                .sum::<f32>()
                                / (end - start) as f32)
                                .sqrt();
                            levels[i] = levels[i] * 0.3 + rms * 0.7;
                        }
                    }
                }
            }

            let _ = sender.send(mono);
        },
        |err| eprintln!("Audio error: {err}"),
        None,
    )?)
}
