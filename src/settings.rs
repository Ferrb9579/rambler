use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::md_action_icons::{MdGetApp, MdInfoOutline, MdSettings, MdSubject};
use dioxus_free_icons::icons::md_content_icons::MdSave;
use dioxus_free_icons::icons::md_hardware_icons::MdKeyboard;
use dioxus_free_icons::icons::md_image_icons::MdPalette;
use dioxus_free_icons::icons::md_navigation_icons::{MdArrowBack, MdRefresh};

use crate::model_download::{self, DownloadEvent};
use crate::preferences::{Preferences, ShortcutControl, ThemePreference};

const VOCABULARY_PLACEHOLDER: &str = "Fanisus\nRambler Desktop\nQwen3-ASR";

#[component]
pub(crate) fn SettingsPage(
    mut preferences: Signal<Preferences>,
    shortcut_control: Signal<ShortcutControl>,
    on_close: EventHandler<()>,
) -> Element {
    let desktop = dioxus::desktop::window();
    let drag_window = desktop.clone();
    let always_on_top_window = desktop.clone();
    let mut vocabulary = use_signal(|| {
        rambler::models::load_vocabulary_source().unwrap_or_else(|error| {
            eprintln!("Could not load vocabulary: {error:#}");
            String::new()
        })
    });
    let mut vocabulary_status = use_signal(String::new);
    let settings_status = use_signal(String::new);
    let mut model_status = use_signal(|| model_download::status().unwrap_or_default());
    let mut downloading = use_signal(|| false);
    let mut download_status = use_signal(String::new);

    rsx! {
        section { class: "settings-page",
            header {
                class: "settings-header",
                onmousedown: move |_| drag_window.drag(),
                button {
                    class: "round-button",
                    aria_label: "Back to Rambler",
                    title: "Back",
                    onmousedown: move |event| event.stop_propagation(),
                    onclick: move |_| on_close.call(()),
                    Icon { width: 21, height: 21, icon: MdArrowBack }
                }
                div { class: "settings-heading",
                    h1 { "Settings" }
                    p { "Rambler runs entirely on this computer" }
                }
                Icon { class: "header-mark", width: 23, height: 23, icon: MdSettings }
            }

            div { class: "settings-content",
                SettingsSection {
                    icon: rsx! { Icon { width: 19, height: 19, icon: MdSubject } },
                    title: "Vocabulary",
                    description: "One canonical name or phrase per line. Changes apply to the next recording.",
                    textarea {
                        class: "vocabulary-editor",
                        aria_label: "Personal vocabulary",
                        spellcheck: false,
                        placeholder: VOCABULARY_PLACEHOLDER,
                        value: vocabulary(),
                        oninput: move |event| {
                            vocabulary.set(event.value());
                            vocabulary_status.set(String::new());
                        }
                    }
                    div { class: "section-actions",
                        span { class: "status-text", "{vocabulary_status}" }
                        button {
                            class: "action-button primary",
                            onclick: move |_| {
                                match rambler::models::save_vocabulary_source(&vocabulary()) {
                                    Ok(()) => vocabulary_status.set("Saved".to_string()),
                                    Err(error) => vocabulary_status.set(format!("{error:#}")),
                                }
                            },
                            Icon { width: 17, height: 17, icon: MdSave }
                            "Save vocabulary"
                        }
                    }
                }

                SettingsSection {
                    icon: rsx! { Icon { width: 19, height: 19, icon: MdGetApp } },
                    title: "Local models",
                    description: "Downloads only the assets required by the model names compiled into Rambler.",
                    div { class: "model-row",
                        div {
                            strong { "Qwen3-ASR + S1-mini" }
                            span { "{model_status().installed_files} of {model_status().total_files} required files" }
                        }
                        span {
                            class: if model_status().installed() { "status-badge installed" } else { "status-badge missing" },
                            if model_status().installed() { "Installed" } else { "Missing files" }
                        }
                    }
                    div { class: "section-actions",
                        span { class: "status-text", "{download_status}" }
                        button {
                            class: "round-button subtle",
                            aria_label: "Recheck model files",
                            title: "Recheck files",
                            disabled: downloading(),
                            onclick: move |_| {
                                match model_download::status() {
                                    Ok(status) => {
                                        model_status.set(status);
                                        download_status.set("Checked".to_string());
                                    }
                                    Err(error) => download_status.set(format!("{error:#}")),
                                }
                            },
                            Icon { width: 18, height: 18, icon: MdRefresh }
                        }
                        button {
                            class: "action-button",
                            disabled: downloading() || model_status().installed(),
                            onclick: move |_| {
                                downloading.set(true);
                                download_status.set("Downloading missing model files…".to_string());
                                let receiver = model_download::start();
                                spawn(async move {
                                    let result = tokio::task::spawn_blocking(move || {
                                        receiver.into_iter().last()
                                    }).await;
                                    downloading.set(false);
                                    match result {
                                        Ok(Some(DownloadEvent::Complete)) => {
                                            model_status.set(model_download::status().unwrap_or_default());
                                            download_status.set("Models installed".to_string());
                                        }
                                        Ok(Some(DownloadEvent::Error(message))) => download_status.set(message),
                                        Ok(Some(DownloadEvent::Progress(message))) => download_status.set(message),
                                        Ok(None) | Err(_) => download_status.set("Download stopped unexpectedly".to_string()),
                                    }
                                });
                            },
                            Icon { width: 17, height: 17, icon: MdGetApp }
                            if downloading() { "Downloading…" } else { "Download missing" }
                        }
                    }
                }

                SettingsSection {
                    icon: rsx! { Icon { width: 19, height: 19, icon: MdPalette } },
                    title: "Appearance",
                    description: "Choose how Rambler looks and behaves above other windows.",
                    div { class: "setting-row stacked",
                        span { "Theme" }
                        div { class: "segmented-control",
                            for (label, theme) in [
                                ("System", ThemePreference::System),
                                ("Light", ThemePreference::Light),
                                ("Dark", ThemePreference::Dark),
                            ] {
                                button {
                                    class: if preferences().theme == theme { "selected" } else { "" },
                                    onclick: move |_| {
                                        let mut next = preferences();
                                        next.theme = theme;
                                        persist_preferences(preferences, shortcut_control, settings_status, next);
                                    },
                                    "{label}"
                                }
                            }
                        }
                    }
                    label { class: "setting-row",
                        div {
                            strong { "Always on top" }
                            span { "Keep the compact toolbar above other windows" }
                        }
                        input {
                            class: "switch",
                            r#type: "checkbox",
                            checked: preferences().always_on_top,
                            onchange: move |event| {
                                let enabled = event.checked();
                                always_on_top_window.window.set_always_on_top(enabled);
                                let mut next = preferences();
                                next.always_on_top = enabled;
                                persist_preferences(preferences, shortcut_control, settings_status, next);
                            }
                        }
                    }
                }

                SettingsSection {
                    icon: rsx! { Icon { width: 19, height: 19, icon: MdKeyboard } },
                    title: "Shortcut",
                    description: "Tap Ctrl once, then press Ctrl again and hold while speaking. Release to submit.",
                    label { class: "setting-row",
                        div {
                            strong { "Global Ctrl shortcut" }
                            span { "Ctrl+C and other normal shortcuts still pass through" }
                        }
                        input {
                            class: "switch",
                            r#type: "checkbox",
                            checked: preferences().shortcut_enabled,
                            onchange: move |event| {
                                let mut next = preferences();
                                next.shortcut_enabled = event.checked();
                                persist_preferences(preferences, shortcut_control, settings_status, next);
                            }
                        }
                    }
                }

                SettingsSection {
                    icon: rsx! { Icon { width: 19, height: 19, icon: MdInfoOutline } },
                    title: "About",
                    description: "A private, CPU-only dictation pipeline with no hosted inference.",
                    div { class: "about-grid",
                        span { "Rambler" }
                        strong { "v{env!(\"CARGO_PKG_VERSION\")}" }
                        span { "Pipeline" }
                        strong { "Audio → ASR → Cleaner → Output" }
                        span { "Models" }
                        strong { "Qwen3-ASR + S1-mini by Superwhisper" }
                    }
                }

                if !settings_status().is_empty() {
                    p { class: "settings-save-status", "{settings_status}" }
                }
            }
        }
    }
}

fn persist_preferences(
    mut preferences: Signal<Preferences>,
    shortcut_control: Signal<ShortcutControl>,
    mut status: Signal<String>,
    next: Preferences,
) {
    shortcut_control.peek().set_enabled(next.shortcut_enabled);
    if let Err(error) = next.save() {
        status.set(format!("Could not save: {error:#}"));
    } else {
        status.set("Saved".to_string());
    }
    preferences.set(next);
}

#[component]
fn SettingsSection(
    icon: Element,
    title: &'static str,
    description: &'static str,
    children: Element,
) -> Element {
    rsx! {
        section { class: "settings-section",
            div { class: "section-title",
                span { class: "section-icon", {icon} }
                div {
                    h2 { "{title}" }
                    p { "{description}" }
                }
            }
            div { class: "section-body", {children} }
        }
    }
}
