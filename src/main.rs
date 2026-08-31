mod controller;
mod ctrl_shortcut;
mod model_download;
mod preferences;
mod settings;
mod style;

use dioxus::desktop::{
    Config, DesktopContext, LogicalSize, WindowBuilder, WindowEvent, tao::event::Event as TaoEvent,
    use_wry_event_handler,
};
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use dioxus_free_icons::icons::md_action_icons::{MdDragIndicator, MdHourglassEmpty, MdSettings};
use dioxus_free_icons::icons::md_av_icons::{MdMic, MdStop};
use dioxus_free_icons::icons::md_content_icons::MdContentCopy;
use dioxus_free_icons::icons::md_navigation_icons::{MdCheck, MdClose, MdMoreVert};

use controller::{Phase, RecordingCommand, RecordingController, RecordingEvent};
use preferences::{Preferences, ShortcutControl};
use settings::SettingsPage;

const WINDOW_WIDTH: f64 = 184.0;
const TOOLBAR_HEIGHT: f64 = 54.0;
const MENU_HEIGHT: f64 = 154.0;
const SETTINGS_WIDTH: f64 = 480.0;
const SETTINGS_HEIGHT: f64 = 640.0;
const WEBVIEW_HEAD: &str = r#"
<meta name="viewport" content="width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no">
<script>
window.addEventListener('keydown', (event) => {
  if ((event.ctrlKey || event.metaKey) && ['+', '-', '=', '0'].includes(event.key)) {
    event.preventDefault();
  }
}, { capture: true });
window.addEventListener('wheel', (event) => {
  if (event.ctrlKey || event.metaKey) event.preventDefault();
}, { passive: false, capture: true });
window.addEventListener('gesturestart', (event) => event.preventDefault(), { passive: false });
</script>
"#;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum AppView {
    #[default]
    Toolbar,
    Settings,
}

fn main() {
    dioxus::LaunchBuilder::desktop()
        .with_cfg(window_config())
        .launch(App);
}

fn window_config() -> Config {
    let window = WindowBuilder::new()
        .with_title("Rambler")
        .with_inner_size(LogicalSize::new(WINDOW_WIDTH, TOOLBAR_HEIGHT))
        .with_resizable(false)
        .with_minimizable(false)
        .with_maximizable(false)
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top(true)
        .with_visible_on_all_workspaces(true);

    #[cfg(target_os = "linux")]
    let window = {
        use dioxus::desktop::tao::platform::unix::WindowBuilderExtUnix;
        window.with_skip_taskbar(true)
    };

    Config::new()
        .with_window(window)
        .with_custom_head(WEBVIEW_HEAD.to_string())
        .with_background_color((0, 0, 0, 0))
        .with_disable_context_menu(true)
}

fn resize_for_menu(window: &DesktopContext, open: bool) {
    let height = if open { MENU_HEIGHT } else { TOOLBAR_HEIGHT };
    window
        .window
        .set_inner_size(LogicalSize::new(WINDOW_WIDTH, height));
}

fn close_menu(mut menu_open: Signal<bool>, window: DesktopContext) {
    menu_open.set(false);
    window.window.request_redraw();
    spawn(async move {
        // WebKit must submit one transparent frame before the Wayland surface
        // shrinks, otherwise the compositor can retain stale menu pixels.
        tokio::time::sleep(std::time::Duration::from_millis(160)).await;
        resize_for_menu(&window, false);
        window.window.request_redraw();
    });
}

fn resize_for_settings(window: &DesktopContext, open: bool) {
    let size = if open {
        LogicalSize::new(SETTINGS_WIDTH, SETTINGS_HEIGHT)
    } else {
        LogicalSize::new(WINDOW_WIDTH, TOOLBAR_HEIGHT)
    };
    window.window.set_inner_size(size);
}

#[allow(non_snake_case)]
fn App() -> Element {
    let controller = use_hook(RecordingController::start);
    let desktop = dioxus::desktop::window();
    let preferences = use_signal(Preferences::load);
    let shortcut_control = use_signal(|| ShortcutControl::new(preferences.peek().shortcut_enabled));
    let mut view = use_signal(AppView::default);
    let mut phase = use_signal(|| Phase::Ready);
    let mut transcript = use_signal(String::new);
    let mut error = use_signal(String::new);
    let mut copied = use_signal(|| false);
    let mut copy_generation = use_signal(|| 0_u64);
    let mut menu_open = use_signal(|| false);

    let shortcut_commands = controller.commands();
    let shortcut_listener_control = shortcut_control.peek().clone();
    use_hook(move || ctrl_shortcut::start(shortcut_commands, shortcut_listener_control));

    let initial_window = desktop.clone();
    use_hook(move || {
        initial_window.set_zoom_level(1.0);
        initial_window
            .window
            .set_always_on_top(preferences.peek().always_on_top);
    });

    let event_controller = controller.clone();
    use_hook(move || {
        spawn(async move {
            loop {
                let controller = event_controller.clone();
                let event = tokio::task::spawn_blocking(move || controller.next_event()).await;
                match event {
                    Ok(Some(RecordingEvent::Phase(next_phase))) => phase.set(next_phase),
                    Ok(Some(RecordingEvent::Transcript(text))) => {
                        transcript.set(text);
                        copied.set(false);
                    }
                    Ok(Some(RecordingEvent::Error(message))) => error.set(message),
                    Ok(None) | Err(_) => break,
                }
            }
        })
    });

    let drag_window = desktop.clone();
    let menu_window = desktop.clone();
    let close_window = desktop.clone();
    let outside_window = desktop.clone();
    let dismiss_window = desktop.clone();
    let blur_window = desktop.clone();
    let settings_window = desktop.clone();
    let settings_back_window = desktop.clone();
    let recording_commands = controller.commands();
    let has_transcript = !transcript().is_empty();

    use_wry_event_handler(move |event, _| {
        if view() == AppView::Toolbar
            && matches!(
                event,
                TaoEvent::WindowEvent {
                    event: WindowEvent::Focused(false) | WindowEvent::CursorLeft { .. },
                    ..
                }
            )
            && menu_open()
        {
            close_menu(menu_open, blur_window.clone());
        }
    });

    rsx! {
        style { {style::APP_CSS} }

        div {
            class: "app-root {preferences().theme.class_name()}",
        if view() == AppView::Settings {
            SettingsPage {
                preferences,
                shortcut_control,
                on_close: move |_| {
                    view.set(AppView::Toolbar);
                    resize_for_settings(&settings_back_window, false);
                }
            }
        } else {
        main {
            class: "app-shell",
            onclick: move |_| {
                if menu_open() {
                    close_menu(menu_open, outside_window.clone());
                }
            },
            nav {
                class: "toolbar",
                aria_label: "Rambler controls",
                button {
                    class: "icon-button drag-handle",
                    aria_label: "Move Rambler",
                    title: "Drag to move",
                    onmousedown: move |_| drag_window.drag(),
                    Icon { width: 24, height: 24, icon: MdDragIndicator }
                }

                div { class: "primary-actions",
                    button {
                        class: if copied() { "icon-button copy-button copied" } else { "icon-button copy-button" },
                        disabled: !has_transcript,
                        aria_label: if copied() { "Transcript copied" } else { "Copy last transcript" },
                        title: if has_transcript { "Copy last transcript" } else { "Nothing to copy yet" },
                        onclick: move |_| {
                            let latest = transcript();
                            if latest.is_empty() {
                                return;
                            }

                            match arboard::Clipboard::new()
                                .and_then(|mut clipboard| clipboard.set_text(latest))
                            {
                                Ok(()) => {
                                    let generation = copy_generation() + 1;
                                    copy_generation.set(generation);
                                    copied.set(true);
                                    spawn(async move {
                                        tokio::time::sleep(std::time::Duration::from_millis(2_500)).await;
                                        if copy_generation() == generation {
                                            copied.set(false);
                                        }
                                    });
                                },
                                Err(copy_error) => {
                                    let message = format!("Could not copy: {copy_error}");
                                    eprintln!("{message}");
                                    error.set(message);
                                }
                            }
                        },
                        if copied() {
                            Icon { width: 21, height: 21, icon: MdCheck }
                        } else {
                            Icon { width: 22, height: 22, icon: MdContentCopy }
                        }
                    }

                    button {
                        class: match phase() {
                            Phase::Ready => "icon-button mic-button",
                            Phase::Recording => "icon-button mic-button recording",
                            Phase::Processing => "icon-button mic-button processing",
                        },
                        disabled: phase() == Phase::Processing,
                        aria_label: match phase() {
                            Phase::Ready => "Start recording",
                            Phase::Recording => "Stop recording",
                            Phase::Processing => "Transcribing",
                        },
                        title: match phase() {
                            Phase::Ready => "Speak · tap Ctrl, then hold Ctrl",
                            Phase::Recording => "Stop recording",
                            Phase::Processing => "Transcribing…",
                        },
                        onclick: move |_| {
                            copied.set(false);
                            error.set(String::new());
                            if recording_commands.send(RecordingCommand::Toggle).is_err() {
                                error.set("The recording controller stopped unexpectedly.".to_string());
                            }
                        },
                        match phase() {
                            Phase::Ready => rsx! { Icon { width: 22, height: 22, icon: MdMic } },
                            Phase::Recording => rsx! { Icon { width: 20, height: 20, icon: MdStop } },
                            Phase::Processing => rsx! { Icon { class: "processing-icon", width: 20, height: 20, icon: MdHourglassEmpty } },
                        }
                    }
                }

                button {
                    class: if menu_open() { "icon-button menu-button active" } else { "icon-button menu-button" },
                    aria_label: if menu_open() { "Close menu" } else { "Open menu" },
                    aria_expanded: menu_open(),
                    title: "Menu",
                    onclick: move |event| {
                        event.stop_propagation();
                        let open = !menu_open();
                        if open {
                            menu_open.set(true);
                            resize_for_menu(&menu_window, true);
                        } else {
                            close_menu(menu_open, menu_window.clone());
                        }
                    },
                    Icon { width: 24, height: 24, icon: MdMoreVert }
                }
            }

            button {
                    class: if menu_open() { "menu-dismiss-layer open" } else { "menu-dismiss-layer closed" },
                    aria_label: "Close menu",
                    aria_hidden: !menu_open(),
                    tabindex: if menu_open() { "0" } else { "-1" },
                    onclick: move |_| {
                        if menu_open() {
                            close_menu(menu_open, dismiss_window.clone());
                        }
                    }
                }
                div {
                    class: if menu_open() { "menu-panel open" } else { "menu-panel closed" },
                    role: "menu",
                    aria_hidden: !menu_open(),
                    onclick: move |event| event.stop_propagation(),
                    button {
                        class: "menu-item",
                        role: "menuitem",
                        onclick: move |_| {
                            menu_open.set(false);
                            view.set(AppView::Settings);
                            resize_for_settings(&settings_window, true);
                        },
                        Icon { width: 18, height: 18, icon: MdSettings }
                        span { "Settings" }
                    }
                    button {
                        class: "menu-item danger",
                        role: "menuitem",
                        onclick: move |_| close_window.close(),
                        Icon { width: 18, height: 18, icon: MdClose }
                        span { "Close" }
                    }
                }

            if !error().is_empty() {
                p { class: "sr-only", role: "alert", "{error}" }
            }
        }
        }
        }
    }
}
