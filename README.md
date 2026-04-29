# herald

Real-time, streaming speech-to-text in the terminal. Runs fully offline using NVIDIA's Nemotron model via ONNX Runtime.

## Download model

```sh
# FP32
hf download altunenes/parakeet-rs --include "nemotron-speech-streaming-en-0.6b/*" --local-dir ./nemotron-speech-streaming-en-0.6b

# INT8
hf download lokkju/nemotron-speech-streaming-en-0.6b-int8 --local-dir ./nemotron-speech-streaming-en-0.6b-int8

# INT4
hf download lokkju/nemotron-speech-streaming-en-0.6b-int4 --local-dir ./nemotron-speech-streaming-en-0.6b-int4
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
