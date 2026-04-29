use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, Stream, StreamConfig};
use parakeet_rs::Nemotron;
use std::time::Duration;
use std::{
    io::Write,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, Sender},
        Arc,
    },
};

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

const MODEL_DIR: &str = "nemotron-speech-streaming-en-0.6b";
const TARGET_HZ: u32 = 16_000;
const CHUNK_SAMPLES: usize = 8_960; // 560ms at 16 kHz

fn main() -> Result<()> {
    // Load model and open audio stream
    let mut model = Nemotron::from_pretrained(MODEL_DIR, None)?;
    let (stream, receiver) = open_input_stream()?;
    stream.play()?;

    // Ctrl+C to cancel stream
    let running = Arc::new(AtomicBool::new(true));
    {
        let running = running.clone();
        ctrlc::set_handler(move || running.store(false, Ordering::SeqCst))?;
    }

    // Every time we get a chunk, transcribe it
    print!("> ");
    std::io::stdout().flush()?;
    let mut buffer = Vec::with_capacity(CHUNK_SAMPLES * 4);

    while running.load(Ordering::SeqCst) {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(samples) => buffer.extend_from_slice(&samples),
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }

        while buffer.len() >= CHUNK_SAMPLES {
            let chunk = buffer.drain(..CHUNK_SAMPLES).collect::<Vec<_>>();
            let text = model
                .transcribe_chunk(&chunk)
                .map_err(|e| format!("Transcription failed: {e}"))?;
            if !text.is_empty() {
                print!("{text}");
                std::io::stdout().flush()?;
            }
        }
    }
    Ok(())
}

fn open_input_stream() -> Result<(Stream, Receiver<Vec<f32>>)> {
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
            let _ = sender.send(mono);
        },
        |err| eprintln!("audio error: {err}"),
        None,
    )?)
}
