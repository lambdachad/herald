//! Minimal Nemotron streaming ASR demo.
//!
//! Records from the default input device, downmixes to mono, resamples to
//! 16 kHz, and feeds 560 ms chunks to `parakeet_rs::Nemotron`. Partial
//! transcript is streamed to stdout. Ctrl+C flushes the model and prints
//! the final transcript.
//!
//! Run from the project root (so `./nemotron` resolves):
//!     nix develop -c cargo run --release

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, Stream, StreamConfig, SupportedStreamConfig};
use parakeet_rs::Nemotron;

const MODEL_DIR: &str = "./nemotron";
const TARGET_SR: u32 = 16_000;
/// 560 ms at 16 kHz — the chunk size Nemotron's cache-aware encoder expects.
const CHUNK_SAMPLES: usize = 8_960;

fn main() -> Result<()> {
    let mut model = Nemotron::from_pretrained(MODEL_DIR, None)
        .map_err(|e| anyhow!("loading nemotron from {MODEL_DIR}: {e}"))?;
    eprintln!("model loaded from {MODEL_DIR}");

    let (stream, supported, audio_rx) = open_input_stream()?;
    eprintln!(
        "input: {} Hz, {} ch, {:?}",
        supported.sample_rate().0,
        supported.channels(),
        supported.sample_format()
    );
    stream.play().context("starting input stream")?;

    let running = Arc::new(AtomicBool::new(true));
    {
        let running = running.clone();
        ctrlc::set_handler(move || running.store(false, Ordering::SeqCst))
            .context("installing Ctrl+C handler")?;
    }

    eprintln!("\n[recording — speak now, Ctrl+C to stop]\n");
    let mut resampler = Resampler::new(supported.sample_rate().0, TARGET_SR);
    let mut buf16k: Vec<f32> = Vec::with_capacity(CHUNK_SAMPLES * 4);

    while running.load(Ordering::SeqCst) {
        let mono = match audio_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(p) => p,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        resampler.push(&mono, &mut buf16k);

        while buf16k.len() >= CHUNK_SAMPLES {
            let chunk: Vec<f32> = buf16k.drain(..CHUNK_SAMPLES).collect();
            let text = model
                .transcribe_chunk(&chunk)
                .map_err(|e| anyhow!("transcribe_chunk: {e}"))?;
            if !text.is_empty() {
                use std::io::Write as _;
                print!("{text}");
                std::io::stdout().flush().ok();
            }
        }
    }

    drop(stream);
    flush(&mut model, &mut buf16k)?;
    println!("\n\n--- final transcript ---");
    println!("{}", model.get_transcript().trim());
    Ok(())
}

/// Open the default input stream and return a receiver of mono `f32` packets.
fn open_input_stream() -> Result<(Stream, SupportedStreamConfig, Receiver<Vec<f32>>)> {
    let device = cpal::default_host()
        .default_input_device()
        .ok_or_else(|| anyhow!("no default input device"))?;
    let supported = device
        .default_input_config()
        .context("querying default input config")?;
    let config: StreamConfig = supported.clone().into();
    let channels = supported.channels() as usize;
    let (tx, rx) = mpsc::channel::<Vec<f32>>();

    let stream = match supported.sample_format() {
        SampleFormat::F32 => build_stream::<f32>(&device, &config, channels, tx)?,
        SampleFormat::I16 => build_stream::<i16>(&device, &config, channels, tx)?,
        SampleFormat::U16 => build_stream::<u16>(&device, &config, channels, tx)?,
        other => return Err(anyhow!("unsupported sample format: {other:?}")),
    };
    Ok((stream, supported, rx))
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    channels: usize,
    tx: Sender<Vec<f32>>,
) -> Result<Stream>
where
    T: Sample + cpal::SizedSample + 'static,
    f32: cpal::FromSample<T>,
{
    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                let mono: Vec<f32> = if channels <= 1 {
                    data.iter().map(|&s| s.to_sample::<f32>()).collect()
                } else {
                    data.chunks_exact(channels)
                        .map(|frame| {
                            frame.iter().map(|&s| s.to_sample::<f32>()).sum::<f32>()
                                / channels as f32
                        })
                        .collect()
                };
                let _ = tx.send(mono);
            },
            |err| eprintln!("cpal stream error: {err}"),
            None,
        )
        .context("building input stream")
}

/// Drain the partial chunk and a few silent chunks so the streaming encoder
/// finalises pending tokens before we read the transcript.
fn flush(model: &mut Nemotron, buf: &mut Vec<f32>) -> Result<()> {
    if !buf.is_empty() {
        buf.resize(CHUNK_SAMPLES, 0.0);
        let text = model
            .transcribe_chunk(buf)
            .map_err(|e| anyhow!("flush partial: {e}"))?;
        if !text.is_empty() {
            print!("{text}");
        }
    }
    let silence = vec![0.0f32; CHUNK_SAMPLES];
    for _ in 0..3 {
        let text = model
            .transcribe_chunk(&silence)
            .map_err(|e| anyhow!("flush silence: {e}"))?;
        if !text.is_empty() {
            print!("{text}");
        }
    }
    Ok(())
}

/// Tiny streaming linear-interpolation resampler. State is one carry sample
/// plus a running fractional read position. Good enough for speech ASR.
struct Resampler {
    ratio: f64,         // output samples per input sample
    pos: f64,           // fractional read position in concatenated input stream
    input_offset: f64,  // absolute index of next packet's first sample
    carry: Option<f32>, // last sample of previous packet
}

impl Resampler {
    fn new(in_sr: u32, out_sr: u32) -> Self {
        Self {
            ratio: out_sr as f64 / in_sr as f64,
            pos: 0.0,
            input_offset: 0.0,
            carry: None,
        }
    }

    fn push(&mut self, packet: &[f32], out: &mut Vec<f32>) {
        if packet.is_empty() {
            return;
        }
        let mut buf: Vec<f32> = Vec::with_capacity(packet.len() + 1);
        if let Some(s) = self.carry {
            buf.push(s);
        }
        buf.extend_from_slice(packet);

        let buf_base = if self.carry.is_some() {
            self.input_offset - 1.0
        } else {
            self.input_offset
        };
        let buf_end = buf_base + buf.len() as f64;
        let step = 1.0 / self.ratio;

        while self.pos + 1.0 < buf_end {
            let local = self.pos - buf_base;
            let i0 = local.floor() as usize;
            let i1 = i0 + 1;
            if i1 >= buf.len() {
                break;
            }
            let frac = (local - local.floor()) as f32;
            out.push(buf[i0] * (1.0 - frac) + buf[i1] * frac);
            self.pos += step;
        }

        self.input_offset += packet.len() as f64;
        self.carry = packet.last().copied();
    }
}
