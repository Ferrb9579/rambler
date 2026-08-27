# Rambler

A minimal, entirely local voice-to-text desktop app written in Rust with Dioxus.

```text
cpal microphone → Whisper base.en Q5_1 → S1-mini by Superwhisper → clipboard
```

There is no Python, JavaScript, web server, external stylesheet, hosted inference API, or runtime model download. The UI, audio capture, local model inference, transcript cleanup, and clipboard integration all run inside one Rust process. The model files stay next to the app in `assets/models/` and are opened locally when the first recording is processed.

## Models

- **Whisper base.en Q5_1** via `whisper-rs` is the default ASR model. It is English-specialized and about 60 MB, giving a much faster CPU turnaround for short dictation while retaining good accuracy. Set `RAMBLER_WHISPER_MODEL=ggml-large-v3-turbo-q5_0.bin` when maximum recognition quality matters more than latency; that optional model is about 574 MB.
- **S1-mini by Superwhisper Q4_K_M** via Candle's pure-Rust Qwen3 runtime cleans the raw transcript. It removes fillers, resolves corrections, restores punctuation/capitalization, and formats spoken numbers, dates, currency, and email addresses. The local quantized model is about 484 MB.

S1-mini is English-only, so the application currently sets Whisper to English. The model files and tokenizer are read from `assets/models/`; transcription does not need Hugging Face, a model cache, or an internet connection.

## Run

Linux needs the usual Dioxus Desktop/WebKit and ALSA development packages. On Ubuntu/Debian, install `libwebkit2gtk-4.1-dev`, `libsoup-3.0-dev`, and `libasound2-dev`. With those installed:

```bash
ulimit -v 25165824
CARGO_BUILD_JOBS=4 CMAKE_BUILD_PARALLEL_LEVEL=4 cargo run --release
```

The `ulimit` caps the app at 24 GiB of virtual memory so a bad model allocation fails instead of exhausting the machine. On Unix, the Rust executable applies the same 24 GiB limit to itself as a second safeguard. WebKit needs this larger virtual-address ceiling during startup even though its resident memory is much smaller. Use a release build: local inference is substantially slower in a debug build.

If compilation is memory constrained, retry with `CARGO_BUILD_JOBS=1 CMAKE_BUILD_PARALLEL_LEVEL=1`.

Whisper uses eight CPU threads by default and the worker loads both local models in the background when the app starts. For a faster but slightly less conservative decoder, set `RAMBLER_WHISPER_GREEDY=1`; use `RAMBLER_WHISPER_THREADS=16` on a machine with enough CPU headroom.

Click the microphone once to record and again to stop. When both local inference stages finish, the cleaned transcript appears with a Copy button.

## End-to-end audio check

The ignored Rust test uses the same worker/channel path as the record button. It can be run against the public 16 kHz speech sample used by whisper.cpp:

```bash
curl --fail --location --output /tmp/rambler-jfk.wav \
  https://raw.githubusercontent.com/ggml-org/whisper.cpp/master/samples/jfk.wav
ulimit -v 25165824
CARGO_BUILD_JOBS=4 CMAKE_BUILD_PARALLEL_LEVEL=4 \
  RAMBLER_E2E_WAV=/tmp/rambler-jfk.wav \
  cargo test --release --test e2e_audio -- --ignored --nocapture
```

The test decodes the downloaded WAV, resamples it if needed, submits samples to the model worker, and asserts that the cleaned transcript is non-empty.

## Project layout

- `src/main.rs` — Dioxus UI and app state
- `src/audio.rs` — native microphone capture and 16 kHz resampling
- `src/models.rs` — local model loading, Whisper inference, and exact S1-mini prompt
- `src/style.rs` — UI styling embedded in Rust
- `assets/models/` — default/optional Whisper, S1-mini, and tokenizer assets used at runtime

Read [MODEL_NOTICE.md](MODEL_NOTICE.md) before redistributing the downloaded model weights.
