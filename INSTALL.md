# Installing Rambler

Rambler is currently developed and tested on 64-bit Ubuntu Linux. It can also
be built on macOS and Windows, with the platform limitations listed below.

## Requirements

- A 64-bit CPU and operating system
- A working microphone
- Rust's current stable toolchain and Cargo
- About 2.8 GB for the included model assets, plus space for the Rust build
- Enough memory to load the ASR and cleaner models; 8 GB of available RAM is a
  practical minimum for the current model pair

Rambler runs inference locally on the CPU. It does not require CUDA, ROCm,
Python, Node.js, a server, or API keys.

The following files must exist before the app starts:

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

Ubuntu 22.04 or newer and Debian 12 or newer provide WebKitGTK 4.1. Install the
compiler, Dioxus desktop dependencies, ALSA headers, and OpenBLAS:

```bash
pkexec apt-get update
pkexec apt-get install build-essential curl wget file pkg-config lld \
  libwebkit2gtk-4.1-dev libxdo-dev libssl-dev \
  libayatana-appindicator3-dev librsvg2-dev \
  libasound2-dev libopenblas-dev
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
pkexec dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel libxdo-devel \
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
pkexec pacman -S --needed base-devel curl wget file openssl \
  webkit2gtk-4.1 appmenu-gtk-module libappindicator-gtk3 librsvg xdotool \
  alsa-lib openblas pkgconf
```

Then install the keyboard rule using the three `pkexec` commands in the
[Ubuntu and Debian section](#ubuntu-and-debian), and run:

```bash
cargo run --release
```

For another Linux distribution, install the equivalents of WebKitGTK 4.1,
xdotool/libxdo, OpenSSL development headers, ALSA development headers,
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
3. The Evergreen WebView2 Runtime if it is not already present. Windows 11
   normally includes it; the installer is available from
   [Microsoft](https://developer.microsoft.com/microsoft-edge/webview2/).

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

Run the checks from the repository root:

```bash
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo run --release
```

On Windows, use the portable feature flags shown above for commands that build
the desktop binary:

```powershell
cargo test --all-targets --no-default-features --features desktop
cargo clippy --all-targets --no-default-features --features desktop -- -D warnings
cargo run --release --no-default-features --features desktop
```

Successful startup opens the small Rambler toolbar. Click the microphone,
speak, click it again, and wait for the processing indicator to finish. The
Copy button becomes enabled after a transcript is produced.

## Troubleshooting

### Linux cannot find WebKitGTK

Confirm that the 4.1 development package is installed. Rambler uses
WebKitGTK 4.1, not the older 4.0 package.

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

The app intentionally does not download models at runtime.

## Upstream platform references

- [Dioxus 0.7 platform dependencies](https://dioxuslabs.com/learn/0.7/getting_started/)
- [Rust and Cargo installation](https://doc.rust-lang.org/stable/cargo/getting-started/installation.html)
- [Microsoft WebView2 Runtime distribution](https://learn.microsoft.com/microsoft-edge/webview2/concepts/distribution)
- [Apple microphone authorization](https://developer.apple.com/documentation/bundleresources/requesting-authorization-for-media-capture-on-macos)
