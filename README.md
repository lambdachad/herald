# herald

Real-time, streaming speech-to-text in the terminal. Runs fully offline using NVIDIA's Nemotron model via ONNX Runtime.

## Setup

### Download the model

```sh
hf download lokkju/nemotron-speech-streaming-en-0.6b-int8 --local-dir ./nemotron
```

### Build and run

```sh
cargo run
```

Speak into your default microphone. Transcribed text streams to stdout in real time. Press `Ctrl+C` to stop.

## How it works

Audio is captured from the default input device at 16 kHz mono, chunked into 560ms segments, and fed to a quantized (int8) ONNX encoder-decoder model. Recognized text is printed incrementally as you speak.

## Dependencies

| Crate | Purpose |
|---|---|
| `parakeet-rs` | Nemotron/Parakeet streaming ASR inference |
| `cpal` | Cross-platform audio capture |
| `anyhow` | Error handling |
| `ctrlc` | Graceful shutdown |
