# Rambler

Rambler is a small, entirely local voice-to-text desktop app written in Rust
and Incular.

See [INSTALL.md](INSTALL.md) for Linux, macOS, and Windows prerequisites and
installation commands.

```text
microphone audio -> ASR model -> cleaner model -> UI/clipboard output
```

Audio capture, inference, cleanup, and clipboard integration run in one process.
There is no hosted inference API, server, Python process, automatic model download,
VAD, silence removal, or custom audio segmentation.

## Architecture

The model layer has three deliberately small pieces:

- `src/models/asr.rs` defines `SpeechRecognizer` and the Qwen3-ASR adapter.
- `src/models/cleaner.rs` owns the S1-mini cleanup model.
- `src/models.rs` connects them as `audio -> ASR -> cleaner -> output` and owns
  the long-lived model worker.

Both models load lazily on the first recording in the background and are reused
for every recording. The cleaner releases its attention cache after each request.
The ASR receives the complete 16 kHz mono recording in one call. The cleaner
receives only the raw transcript and optional canonical vocabulary.

## Selecting an ASR model

Change `ASR_MODEL_NAME` in `src/models/config.rs`, then rebuild. Model selection
is intentionally compile-time only; there is no runtime picker or environment
variable.

Qwen3-ASR checkpoints use the existing adapter, so switching between compatible
Qwen sizes only requires placing the checkpoint under `assets/models/<name>/`
and changing that constant. ZipFormer XL and Parakeet are different model
architectures and checkpoint formats, not interchangeable Qwen weight names.
Trying either family requires adding one `SpeechRecognizer` implementation and
one factory arm in `src/models/asr.rs`; the recorder, pipeline, cleaner, worker,
UI, and E2E test do not need to change.

The currently selected assets are:

- `assets/models/qwen3-asr-0.6b/` for Qwen3-ASR-0.6B
- `assets/models/s1-mini-q4_k_m.gguf` for cleanup
- `assets/models/s1-mini-tokenizer.json` for the cleaner tokenizer

## Vocabulary

Add one canonical name, technical term, or phrase per non-comment line in
`assets/models/personal_dictionary.tsv`:

```text
Fanisus
Rambler Desktop
Qwen3-ASR
```

Vocabulary is applied in both useful places:

1. Qwen receives it as a canonical-spelling prompt.
2. S1-mini receives it as a constrained cleanup control line, so a future ASR
   backend without prompting can still correct likely spellings.

The cleaner is instructed to use a term only when the transcript supports it,
not to inject unrelated vocabulary into the output.

## CPU performance

Always use a native release build. Debug inference is not usable for this model.

On Ubuntu/Debian, install the native desktop/ALSA dependencies and OpenBLAS:

```bash
pkexec apt-get install build-essential pkg-config libasound2-dev libopenblas-dev \
  libxkbcommon-dev libwayland-dev libvulkan-dev
```

Then run the app:

```bash
cargo run --release
```

The repository's Cargo configuration applies native x86-64 CPU tuning, so no
shell variables or wrapper command are required.

The default `qwen-blas` Cargo feature enables OpenBLAS on Linux (Accelerate on
macOS). Rambler pins each OpenBLAS call to one internal thread because Qwen
already distributes matrix work over its own pool; this avoids nested thread
oversubscription. Qwen uses up to 16 workers, configured by the compile-time
`ASR_CPU_THREADS` constant in `src/models/config.rs`.

One release-mode run on this 16-core Ryzen laptop with the included 11-second
JFK fixture measured:

| Build | Qwen ASR | S1 cleanup | Total |
| --- | ---: | ---: | ---: |
| Native, pure Rust, 8 workers | 78.82 s | 0.80 s | 79.62 s |
| Native, OpenBLAS, 16 workers | 1.22 s | 0.82 s | 2.04 s |

That is about a 65x ASR latency reduction for this fixture. Treat it as a local
before/after measurement, not a cross-machine benchmark.

For a portable build without a BLAS dependency, disable default features and
explicitly restore the desktop feature. This is substantially slower on x86:

```bash
cargo run --release --no-default-features --features desktop
```

The 573 MB `qwen-asr-int8.sidecar` speeds warm model loading and reduces
duplicated memory, but qwen-asr 0.11's INT8 execution kernels are ARM-specific;
it does not make Qwen inference INT8 on this x86-64 machine.

## Run

Click the microphone once to record and again to stop. For keyboard
push-to-talk, tap either Ctrl key once, then press Ctrl again and keep holding
while you speak. Releasing the second press stops and submits the recording. A
single hold or plain double-tap does nothing, and normal shortcuts such as
Ctrl+C pass through unchanged. After ASR and cleanup finish, the cleaned
transcript is available from the Copy button.

Buttons have a small hover lift and press response. A successful copy blurs and
shrinks the icon, pops in a green check for 2.4 seconds, then restores the copy
icon without moving the toolbar. Repeated copies restart the feedback; a new
recording or transcript cancels it. Recording gently pulses, processing rotates
the hourglass, page changes fade in, and successful vocabulary saves and model
rechecks briefly show a green check. These effects respect the system's reduced
motion preference, and activity animations stop when the app returns to Ready.

Click the **Settings** icon to edit vocabulary, repair or
download missing files for the compiled-in models, select a theme, enable or
disable the global shortcut, change always-on-top behavior, and view build and
model information. Preferences persist in `assets/settings.json`; vocabulary
changes apply to the next recording without reloading the models.

On Linux, the global Ctrl gesture reads keyboard events without grabbing or
blocking them. Install its device-access rule once:

```bash
pkexec install -m 0644 packaging/72-rambler-keyboard.rules /etc/udev/rules.d/72-rambler-keyboard.rules
pkexec udevadm control --reload-rules
pkexec udevadm trigger --subsystem-match=input
```

The rule is already installed on the development machine. No environment
variables or elevated privileges are needed when running Rambler afterward.

Set `RAMBLER_PROFILE=1` when measuring model load, ASR, cleanup, and total
latency. `RAMBLER_DEBUG_TRANSCRIPT=1` prints the raw ASR text. These flags do not
select or alter the model pipeline.

## End-to-end audio check

On Linux, use the constrained runner for builds and tests. It places the entire
command and its descendants in a systemd user cgroup with a **6 GiB soft limit,
8 GiB hard limit, no swap, and a 20-minute timeout**. Cargo defaults to two build
jobs; tests run serially so they cannot load multiple model pairs at once.
The runner reports peak cgroup memory, including subprocesses and file cache,
and fails if a systemd user session is unavailable rather than running uncapped.

```bash
scripts/test-constrained.sh
scripts/test-constrained.sh cargo clippy --all-targets --locked -- -D warnings
```

The ignored Rust test decodes and resamples a WAV, submits it through the same
worker/channel path as the record button, and requires non-empty cleaned output:

```bash
RAMBLER_E2E_WAV=/path/to/speech.wav \
  RAMBLER_PROFILE=1 \
  scripts/test-constrained.sh cargo test --release --locked \
    --no-default-features --features qwen-blas --test e2e_audio \
    speech_pipeline -- --ignored --nocapture
```

This runs both the single-recording check and a five-recording memory regression
check. After two warmup requests, RSS may grow by at most 128 MiB; the cgroup
enforces the absolute memory budget throughout model loading and inference.
Each worker reply also has a five-minute timeout. The headless feature flags
avoid compiling or loading Incular and GPU dependencies for model-only tests.

Override the limits explicitly for another machine with
`RAMBLER_TEST_MEMORY_HIGH`, `RAMBLER_TEST_MEMORY_MAX`, and
`RAMBLER_TEST_TIMEOUT` (for example `4G`, `6G`, and `10m`). These limits apply
only to the command launched by the runner. On other platforms, run Cargo
directly with equivalent OS resource controls; the Linux runner is required
for the hard limits above.

To exercise five native toolbar/settings resize roundtrips and save a settings
screenshot without loading the models:

```bash
RAMBLER_CAPTURE=/tmp/rambler-settings.png RAMBLER_CAPTURE_SETTINGS=1 \
  RAMBLER_CAPTURE_ROUNDTRIP=1 RAMBLER_CAPTURE_EXIT=1 \
  scripts/test-constrained.sh cargo run --locked
```

To capture the copy animation for visual QA, add `RAMBLER_CAPTURE_COPY=1` and
`RAMBLER_CAPTURE_SEQUENCE=/tmp/rambler-copy-frames` to the capture command
(omit `RAMBLER_CAPTURE_SETTINGS` and `RAMBLER_CAPTURE_ROUNDTRIP`). This previews
the success animation without changing the clipboard or loading model weights.

Additional ignored tests in `tests/e2e_audio.rs` cover vocabulary names and a
larger fixture matrix when those WAV directories are available.

## Project layout

- `src/audio.rs` - microphone capture, mono conversion, and 16 kHz resampling
- `src/models.rs` - worker and simple end-to-end model pipeline
- `src/models/` - ASR adapter, cleaner, config, paths, and vocabulary
- `src/main.rs` - Incular UI, native window controls, and worker hand-off
- `src/preferences.rs` - persistent settings
- `tests/ui/` - window lifecycle and settings navigation checks
- `tests/e2e_audio.rs` - real worker-path audio tests
- `scripts/test-constrained.sh` - Linux memory- and time-limited command runner
- `assets/models/` - local model assets and vocabulary
