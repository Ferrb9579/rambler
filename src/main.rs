mod models;
mod style;

use std::sync::mpsc;

use dioxus::prelude::*;
use models::{InferenceJob, start_model_worker};
use rambler::audio;

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Ready,
    Recording,
    Processing,
}

fn main() {
    sanitize_host_environment();
    cap_memory();
    dioxus::launch(App);
}

#[cfg(unix)]
fn sanitize_host_environment() {
    // When launched from a Snap-packaged IDE, GTK/GIO can discover the IDE's
    // private libraries and mix them with the host WebKit stack. Remove those
    // loader hints before Dioxus initializes GTK.
    if std::env::var_os("SNAP").is_some() {
        for key in [
            "GDK_PIXBUF_MODULEDIR",
            "GDK_PIXBUF_MODULE_FILE",
            "GIO_MODULE_DIR",
            "GIO_LAUNCHED_DESKTOP_FILE",
            "GIO_LAUNCHED_DESKTOP_FILE_PID",
            "GSETTINGS_SCHEMA_DIR",
            "GTK_EXE_PREFIX",
            "GTK_IM_MODULE_FILE",
            "GTK_MODULES",
            "GTK_PATH",
            "LOCPATH",
            "SNAP",
            "SNAP_ARCH",
            "SNAP_COMMON",
            "SNAP_CONTEXT",
            "SNAP_COOKIE",
            "SNAP_DATA",
            "SNAP_EUID",
            "SNAP_INSTANCE_NAME",
            "SNAP_LAUNCHER_ARCH_TRIPLET",
            "SNAP_LIBRARY_PATH",
            "SNAP_NAME",
            "SNAP_REAL_HOME",
            "SNAP_REVISION",
            "SNAP_UID",
            "SNAP_USER_COMMON",
            "SNAP_USER_DATA",
            "SNAP_VERSION",
            "XDG_CONFIG_DIRS",
            "XDG_CONFIG_DIRS_VSCODE_SNAP_ORIG",
            "XDG_DATA_DIRS",
            "XDG_DATA_DIRS_VSCODE_SNAP_ORIG",
            "XDG_DATA_HOME",
        ] {
            // This runs before Dioxus starts any threads.
            unsafe { std::env::remove_var(key) };
        }

        // Snap's graphics sandbox can leave GTK/WebKit without a usable GL
        // context. Software compositing keeps the app launchable in that
        // environment; normal desktop launches are unaffected.
        unsafe {
            std::env::set_var("GDK_BACKEND", "x11");
            std::env::set_var("LIBGL_ALWAYS_SOFTWARE", "1");
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }
    }
}

#[cfg(not(unix))]
fn sanitize_host_environment() {}

#[cfg(unix)]
fn cap_memory() {
    // WebKit reserves a large virtual address range during startup. Keep a
    // generous but finite ceiling so those mappings do not make the process
    // unbounded while still leaving room for the local models.
    const TWENTY_FOUR_GIB: u64 = 24 * 1024 * 1024 * 1024;
    if let Ok((_, hard_limit)) = rlimit::getrlimit(rlimit::Resource::AS) {
        let soft_limit = hard_limit.min(TWENTY_FOUR_GIB);
        if let Err(error) = rlimit::setrlimit(rlimit::Resource::AS, soft_limit, hard_limit) {
            eprintln!("warning: could not set the 24 GiB memory limit: {error}");
        }
    }
}

#[cfg(not(unix))]
fn cap_memory() {}

#[allow(non_snake_case)]
fn App() -> Element {
    let worker = use_hook(start_model_worker);
    let mut phase = use_signal(|| Phase::Ready);
    let mut transcript = use_signal(String::new);
    let mut error = use_signal(String::new);
    let mut copied = use_signal(|| false);

    let status = match phase() {
        Phase::Ready if transcript().is_empty() => "Click to start speaking",
        Phase::Ready => "Ready for another thought",
        Phase::Recording => "Listening… click when you’re done",
        Phase::Processing => "Transcribing and polishing your words…",
    };

    rsx! {
        style { {style::APP_CSS} }

        main { class: "shell",
            section { class: "app-card",
                header { class: "brand",
                    div { class: "brand-mark", "R" }
                    span { "Rambler" }
                }

                div { class: "hero",
                    p { class: "eyebrow", "VOICE NOTES, CLEANED UP" }
                    h1 { "Say it. Get the words." }
                    p { class: "intro", "One click to record. One click to turn your voice into clear, copy-ready text." }
                }

                div { class: "recorder",
                    button {
                        class: if phase() == Phase::Recording { "record-button recording" } else { "record-button" },
                        disabled: phase() == Phase::Processing,
                        aria_label: if phase() == Phase::Recording { "Stop recording" } else { "Start recording" },
                        onclick: move |_| {
                            copied.set(false);
                            error.set(String::new());

                            if phase() == Phase::Recording {
                                match audio::stop_recording() {
                                    Ok(samples) if samples.len() >= 1_600 => {
                                        phase.set(Phase::Processing);
                                        let (reply_tx, reply_rx) = mpsc::channel();
                                        let job = InferenceJob { samples, reply: reply_tx };
                                        if worker.send(job).is_err() {
                                            error.set("The model worker stopped unexpectedly.".to_string());
                                            phase.set(Phase::Ready);
                                            return;
                                        }

                                        spawn(async move {
                                            let result = tokio::task::spawn_blocking(move || reply_rx.recv()).await;
                                            match result {
                                                Ok(Ok(Ok(text))) => transcript.set(text),
                                                Ok(Ok(Err(message))) => error.set(message),
                                                _ => error.set("The model worker did not respond.".to_string()),
                                            }
                                            phase.set(Phase::Ready);
                                        });
                                    }
                                    Ok(_) => error.set("That recording was too short. Try speaking for a moment longer.".to_string()),
                                    Err(message) => error.set(message),
                                }
                            } else {
                                match audio::start_recording() {
                                    Ok(()) => {
                                        transcript.set(String::new());
                                        phase.set(Phase::Recording);
                                    }
                                    Err(message) => error.set(message),
                                }
                            }
                        },
                        if phase() == Phase::Recording {
                            span { class: "stop-icon" }
                        } else if phase() == Phase::Processing {
                            span { class: "spinner" }
                        } else {
                            svg {
                                view_box: "0 0 24 24",
                                "aria-hidden": "true",
                                path { d: "M12 15.5a3.5 3.5 0 0 0 3.5-3.5V5a3.5 3.5 0 1 0-7 0v7a3.5 3.5 0 0 0 3.5 3.5Zm-1.8-10.5a1.8 1.8 0 1 1 3.6 0v7a1.8 1.8 0 1 1-3.6 0V5Zm-5 6.7a.85.85 0 1 1 1.7 0 5.1 5.1 0 0 0 10.2 0 .85.85 0 1 1 1.7 0 6.8 6.8 0 0 1-5.95 6.74V21h2.2a.85.85 0 1 1 0 1.7h-6.1a.85.85 0 1 1 0-1.7h2.2v-2.56A6.8 6.8 0 0 1 5.2 11.7Z" }
                            }
                        }
                    }
                    div { class: "status-row",
                        if phase() == Phase::Recording { span { class: "live-dot" } }
                        p { class: "status", "{status}" }
                    }
                }

                if !error().is_empty() {
                    div { class: "error", role: "alert", span { "{error}" } }
                }

                if !transcript().is_empty() {
                    section { class: "result",
                        div { class: "result-heading",
                            span { "TRANSCRIPT" }
                            button {
                                class: "copy-button",
                                onclick: move |_| {
                                    match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(transcript())) {
                                        Ok(()) => copied.set(true),
                                        Err(copy_error) => error.set(format!("Could not copy: {copy_error}")),
                                    }
                                },
                                if copied() { "✓  Copied" } else { "Copy" }
                            }
                        }
                        p { class: "transcript", "{transcript}" }
                    }
                }

                footer {
                    span { class: "privacy-dot" }
                    "Runs locally · Whisper large-v3-turbo · S1-mini by Superwhisper"
                }
            }
        }
    }
}
