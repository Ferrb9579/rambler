# Installing Rambler

Rambler is currently developed and tested on 64-bit Ubuntu Linux. It can also
be built on macOS and Windows, with the platform limitations listed below.

## Requirements

- A 64-bit CPU and operating system
- A working microphone
- Rust's current stable toolchain and Cargo
- The current Incular checkout beside Rambler at `../incular` (Cargo uses local
  path dependencies)
- A desktop session and a compatible Vulkan, OpenGL, Metal, or DirectX GPU driver
- About 2.8 GB for the included model assets, plus space for the Rust build
- Enough memory to load the ASR and cleaner models; 8 GB of available RAM is a
  practical minimum for the current model pair

Rambler runs inference locally on the CPU. It does not require CUDA, ROCm,
Python, Node.js, a server, or API keys.

The following files must exist before transcription can run. The app can open
without them; use **Settings → Download missing** to install them:

```text
assets/models/qwen3-asr-0.6b/model.safetensors
assets/models/s1-mini-q4_k_m.gguf
assets/models/s1-mini-tokenizer.json
```

Keep the entire `assets/models/qwen3-asr-0.6b/` directory; the tokenizer,
configuration, and sidecar files in it are also required. Model selection is
described in the main [README](README.md#selecting-an-asr-model).

## Install Rust

Use [rustup](https://rustup.rs/) so that Cargo and the stable Rust compiler are
installed together.

Linux or macOS:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
```

On Windows, download and run `rustup-init.exe` from the same website. Select
the default MSVC toolchain. Open a new terminal after installation, then check:

```text
rustc --version
cargo --version
```

## Ubuntu and Debian

Install the compiler, Incular native desktop dependencies, ALSA headers, and
OpenBLAS:

```bash
pkexec apt-get update
pkexec apt-get install build-essential curl pkg-config lld \
  libxkbcommon-dev libwayland-dev libvulkan-dev libasound2-dev libopenblas-dev
```

Install the one-time keyboard access rule used by the global Ctrl gestures:

```bash
pkexec install -m 0644 packaging/72-rambler-keyboard.rules \
  /etc/udev/rules.d/72-rambler-keyboard.rules
pkexec udevadm control --reload-rules
pkexec udevadm trigger --subsystem-match=input
```

If Ctrl gestures are unavailable immediately afterward, log out and back in or
reboot once. The microphone button does not depend on this rule.

Build and run from the repository root:

```bash
cargo run --release
```

No environment variables, `sudo`, `pkexec`, wrapper scripts, or resource-limit
changes are needed when running the app.

## Fedora

```bash
pkexec dnf install curl libxkbcommon-devel wayland-devel vulkan-loader-devel \
  alsa-lib-devel openblas-devel pkgconf-pkg-config
pkexec dnf group install c-development
```

Then install the keyboard rule using the three `pkexec` commands in the
[Ubuntu and Debian section](#ubuntu-and-debian), and run:

```bash
cargo run --release
```

## Arch Linux and Manjaro

```bash
pkexec pacman -Syu
pkexec pacman -S --needed base-devel curl libxkbcommon wayland vulkan-icd-loader \
  alsa-lib openblas pkgconf
```

Then install the keyboard rule using the three `pkexec` commands in the
[Ubuntu and Debian section](#ubuntu-and-debian), and run:

```bash
cargo run --release
```

For another Linux distribution, install the equivalents of libxkbcommon,
Wayland development headers, a Vulkan/OpenGL driver, ALSA development headers,
OpenBLAS, a C/C++ build toolchain, and `pkg-config`.

## macOS

Install Apple's command-line developer tools:

```bash
xcode-select --install
```

Install Rust as described above, then run from the repository root:

```bash
cargo run --release
```

The optimized Qwen path uses Apple's built-in Accelerate framework, so a
separate OpenBLAS installation is not needed. macOS will ask for microphone
access the first time Rambler records; allow it under **System Settings →
Privacy & Security → Microphone** if it was previously denied.

Current limitation: the global tap-then-hold Ctrl gesture is implemented only
on Linux. On macOS, use the microphone button.

## Windows 10 and 11

Install:

1. Visual Studio 2022 Build Tools with **Desktop development with C++** and a
   Windows SDK.
2. Rust using `rustup-init.exe` and the stable MSVC toolchain.
3. A current graphics driver supporting DirectX 12, Vulkan, or OpenGL.

The current BLAS-optimized Qwen build supports Linux OpenBLAS and macOS
Accelerate. On Windows, use the portable CPU fallback:

```powershell
cargo run --release --no-default-features --features desktop
```

Allow microphone access when Windows asks. If it was denied, enable it under
**Settings → Privacy & security → Microphone**.

Current limitations: the Windows fallback is substantially slower than the
Linux OpenBLAS build, and the global tap-then-hold Ctrl gesture is currently
Linux-only. The microphone button remains available.

## Verify the installation

On Linux with a systemd user session, run checks inside the memory-limited
runner. It caps the entire command at 8 GiB RAM with no swap and a 20-minute
timeout, uses two build jobs, and runs tests serially:

```bash
scripts/test-constrained.sh
scripts/test-constrained.sh cargo clippy --all-targets --locked -- -D warnings
cargo run --release
```

On macOS, run Cargo checks directly. On Windows, also use the portable feature
flags shown above for commands that build the desktop binary:

```powershell
cargo test --all-targets --no-default-features --features desktop
cargo clippy --all-targets --no-default-features --features desktop -- -D warnings
cargo run --release --no-default-features --features desktop
```

Successful startup opens the small Rambler toolbar. Click the microphone,
speak, click it again, and wait for the processing indicator to finish. The
Copy button becomes enabled after a transcript is produced.

## Troubleshooting

### The desktop window cannot start

Run from a graphical desktop session and check that a compatible Vulkan or
OpenGL driver is installed. Incular draws the UI through WGPU.

### A test exceeds memory or hangs

Use `scripts/test-constrained.sh` on Linux. Adjust
`RAMBLER_TEST_MEMORY_HIGH`, `RAMBLER_TEST_MEMORY_MAX`, and
`RAMBLER_TEST_TIMEOUT` only when the machine has enough available memory.
See the [audio and memory checks](README.md#end-to-end-audio-check) for model
tests that reuse one worker and report memory after each request.

### Linux reports that the Ctrl shortcut is unavailable

Check that the rule is installed:

```bash
test -f /etc/udev/rules.d/72-rambler-keyboard.rules && echo installed
```

Reload the rule using the commands above, then log out and back in. Do not run
Rambler itself as root.

### OpenBLAS cannot be found

Install the OpenBLAS development package for the distribution, then rebuild:

```bash
cargo clean -p qwen-asr
cargo run --release
```

### Microphone recording fails

Check the operating system's microphone privacy settings and confirm that an
input device is selected. On Linux, verify that the desktop's sound settings
can record from the same device before debugging Rambler.

### Model assets are missing

Run this from the repository root:

```bash
test -f assets/models/qwen3-asr-0.6b/model.safetensors
test -f assets/models/s1-mini-q4_k_m.gguf
test -f assets/models/s1-mini-tokenizer.json
```

Open **Settings → Download missing** to fetch required assets, then record
again. Downloads run only when explicitly requested.

## Upstream platform references

- [Incular installation](https://github.com/FabortLabs/incular/blob/master/docs/content/docs/introduction/installation.mdx)
- [Rust and Cargo installation](https://doc.rust-lang.org/stable/cargo/getting-started/installation.html)
- [Apple microphone authorization](https://developer.apple.com/documentation/bundleresources/requesting-authorization-for-media-capture-on-macos)
