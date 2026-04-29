# herald

Real-time, streaming speech-to-text in the terminal. Runs fully offline using NVIDIA's Nemotron model via ONNX Runtime.

## Download the model

```sh
hf download altunenes/parakeet-rs --include "nemotron-speech-streaming-en-0.6b/*" --local-dir ./
```

## Build and run

```sh
cargo run
```

Speak into your default microphone. Transcribed text streams to stdout in real time. Press `Ctrl+C` to stop.

## Dependencies

| Crate | Purpose |
|---|---|
| `cpal` | Cross-platform audio capture |
| `parakeet-rs` | Nemotron streaming ASR inference |
