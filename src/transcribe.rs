use crate::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, Stream, StreamConfig};
use parakeet_rs::Nemotron;
use std::sync::mpsc::{self, Receiver, Sender};

// Constants
const TARGET_HZ: u32 = 16_000;
const CHUNK_SAMPLES: usize = 8_960; // 560ms at 16 kHz

pub struct Transcriber {
    model: Nemotron,
    stream: Option<Stream>,
    audio_rx: Option<Receiver<Vec<f32>>>,
    buffer: Vec<f32>,
}

impl Transcriber {
    pub fn new() -> Result<Self> {
        let model = Nemotron::from_pretrained("nemotron-speech-streaming-en-0.6b", None)?;
        Ok(Self {
            model,
            stream: None,
            audio_rx: None,
            buffer: Vec::with_capacity(CHUNK_SAMPLES * 4),
        })
    }

    pub fn start_capture(&mut self) -> Result<()> {
        let (stream, receiver) = open_input_stream()?;
        stream.play()?;
        self.stream = Some(stream);
        self.audio_rx = Some(receiver);
        self.buffer.clear();
        Ok(())
    }

    pub fn stop_capture(&mut self) {
        self.stream = None;
        self.audio_rx = None;
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

        // Process all full chunks
        while self.buffer.len() >= CHUNK_SAMPLES {
            let chunk: Vec<f32> = self.buffer.drain(..CHUNK_SAMPLES).collect();
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

    pub fn is_recording(&self) -> bool {
        self.stream.is_some()
    }
}

fn open_input_stream() -> Result<(Stream, Receiver<Vec<f32>>)> {
    // Find default input device
    let device = cpal::default_host()
        .default_input_device()
        .ok_or("No input device found")?;

    // Get compatible config
    let supported = device
        .supported_input_configs()?
        .filter(|c| c.min_sample_rate() <= TARGET_HZ && c.max_sample_rate() >= TARGET_HZ)
        .min_by_key(|c| {
            (
                c.channels() as u32,
                (c.sample_format() != SampleFormat::F32) as u32,
            )
        })
        .ok_or(format!("Device does not support {TARGET_HZ} Hz"))?
        .with_sample_rate(TARGET_HZ);

    // Open input stream
    let channels = supported.channels() as usize;
    let format = supported.sample_format();
    let config: StreamConfig = supported.into();
    let (sender, receiver) = mpsc::channel();
    let stream = match format {
        SampleFormat::F32 => build_stream::<f32>(&device, &config, channels, sender)?,
        SampleFormat::I16 => build_stream::<i16>(&device, &config, channels, sender)?,
        SampleFormat::U16 => build_stream::<u16>(&device, &config, channels, sender)?,
        other => return Err(format!("Unsupported sample format: {other:?}").into()),
    };
    Ok((stream, receiver))
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    channels: usize,
    sender: Sender<Vec<f32>>,
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
                    .map(|&s| s.to_sample::<f32>())
                    .collect::<Vec<_>>()
            } else {
                data.chunks_exact(channels)
                    .map(|frame| {
                        frame.iter().map(|&s| s.to_sample::<f32>()).sum::<f32>() / channels as f32
                    })
                    .collect::<Vec<_>>()
            };
            let _ = sender.send(mono);
        },
        |err| eprintln!("audio error: {err}"),
        None,
    )?)
}
