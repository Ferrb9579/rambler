mod controller;
mod ctrl_shortcut;
mod model_download;
mod motion;
mod preferences;

#[cfg(test)]
#[path = "../tests/ui/mod.rs"]
mod ui_tests;

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::mpsc::Sender,
};

use incular::config::{Clip, TransparencyMode, WindowSizePolicy};
use incular::controls::{Switch, TextArea, TextFieldStyle};
use incular::prelude::*;
use incular::reactive::{Action, ActionState};

use controller::{Phase, RecordingCommand, RecordingController, RecordingEvent};
use motion::Motion;
use preferences::{Preferences, ShortcutControl, ThemePreference};

// Shared four-point spacing scale for both windows.
const SPACE_XS: f32 = 4.0;
const SPACE_SM: f32 = 8.0;
const SPACE_MD: f32 = 12.0;
const SPACE_LG: f32 = 16.0;
const WINDOW_INSET: f32 = SPACE_XS;
const ROUND_BUTTON: f32 = 40.0;
const ACTION_HEIGHT: f32 = 36.0;
const TOOLBAR_ICON: f32 = 24.0;
const ACTION_ICON: f32 = 20.0;
const TOOLBAR: Size = Size {
    width: ROUND_BUTTON * 4.0 + SPACE_SM * 3.0 + SPACE_SM * 2.0 + WINDOW_INSET * 2.0,
    height: ROUND_BUTTON + SPACE_SM * 2.0 + WINDOW_INSET * 2.0,
};
const SETTINGS: Size = Size {
    width: 480.0,
    height: 640.0,
};

#[derive(Clone, Copy, Default, PartialEq)]
enum View {
    #[default]
    Toolbar,
    Settings,
}

#[derive(Clone)]
struct State {
    view: Signal<View>,
    phase: Signal<Phase>,
    transcript: Signal<String>,
    error: Signal<String>,
    motion: Motion,
    clipboard: Rc<RefCell<Option<arboard::Clipboard>>>,
    preferences: Signal<Preferences>,
    shortcut: ShortcutControl,
    vocabulary: TextEditingController,
    vocabulary_status: Signal<String>,
    settings_status: Signal<String>,
    model_status: Signal<model_download::ModelStatus>,
    download_status: Signal<String>,
    download: Action<(), String, String>,
}

#[derive(Clone)]
struct UiWindow {
    handle: Rc<RefCell<Option<WindowHandle>>>,
    state: State,
    icons: Icons,
    commands: Sender<RecordingCommand>,
}

impl State {
    fn new() -> Self {
        let preferences = Preferences::load();
        Self {
            view: Signal::new(View::Toolbar),
            phase: Signal::new(Phase::Ready),
            transcript: Signal::new(String::new()),
            error: Signal::new(String::new()),
            motion: Motion::default(),
            clipboard: Rc::new(RefCell::new(None)),
            shortcut: ShortcutControl::new(preferences.shortcut_enabled),
            preferences: Signal::new(preferences),
            vocabulary: TextEditingController::with_text(
                rambler::models::load_vocabulary_source().unwrap_or_default(),
            ),
            vocabulary_status: Signal::new(String::new()),
            settings_status: Signal::new(String::new()),
            model_status: Signal::new(model_download::status().unwrap_or_default()),
            download_status: Signal::new(String::new()),
            download: Action::new(|()| async move {
                tokio::task::spawn_blocking(|| {
                    let receiver = model_download::start();
                    let mut latest_progress = String::new();
                    for event in receiver {
                        match event {
                            model_download::DownloadEvent::Progress(message) => {
                                latest_progress = message;
                            }
                            model_download::DownloadEvent::Complete => {
                                return Ok("Models installed".to_string());
                            }
                            model_download::DownloadEvent::Error(message) => return Err(message),
                        }
                    }
                    Err(if latest_progress.is_empty() {
                        "Download stopped unexpectedly".to_string()
                    } else {
                        format!("Download stopped after: {latest_progress}")
                    })
                })
                .await
                .map_err(|error| format!("Download task failed: {error}"))?
            }),
        }
    }
}

#[derive(Clone, Copy)]
struct Palette {
    text: Color,
    muted: Color,
    surface: Color,
    raised: Color,
    soft: Color,
    strong: Color,
    border: Color,
    accent: Color,
    accent_soft: Color,
    danger: Color,
    success: Color,
    success_soft: Color,
    dark: bool,
}

impl Palette {
    fn light() -> Self {
        Self {
            text: rgb(0x17, 0x18, 0x17),
            muted: rgb(0x6f, 0x71, 0x6f),
            surface: Color::WHITE,
            raised: Color::WHITE,
            soft: rgb(0xf2, 0xf3, 0xf2),
            strong: rgb(0xe8, 0xe9, 0xe8),
            border: Color::rgba(0, 0, 0, 23),
            accent: rgb(0x31, 0x5f, 0xdb),
            accent_soft: rgb(0xe8, 0xee, 0xff),
            danger: rgb(0xb1, 0x2f, 0x2a),
            success: rgb(0x27, 0x7c, 0x43),
            success_soft: rgb(0xe5, 0xf3, 0xe9),
            dark: false,
        }
    }

    fn dark() -> Self {
        Self {
            text: rgb(0xf1, 0xf3, 0xf1),
            muted: rgb(0xa7, 0xaa, 0xa7),
            surface: rgb(0x20, 0x22, 0x20),
            raised: rgb(0x29, 0x2b, 0x29),
            soft: rgb(0x2d, 0x30, 0x2d),
            strong: rgb(0x35, 0x38, 0x35),
            border: Color::rgba(255, 255, 255, 28),
            accent: rgb(0x91, 0xac, 0xff),
            accent_soft: rgb(0x29, 0x37, 0x5f),
            danger: rgb(0xff, 0x93, 0x8b),
            success: rgb(0x7b, 0xd5, 0x98),
            success_soft: rgb(0x25, 0x3d, 0x2d),
            dark: true,
        }
    }
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::rgba(r, g, b, 255)
}

#[derive(Clone)]
struct ThemedIcon {
    light: ImageHandle,
    dark: ImageHandle,
}

impl ThemedIcon {
    fn load(bytes: &'static [u8]) -> Self {
        Self::with_colors(bytes, [23, 24, 23], [241, 243, 241])
    }

    fn with_colors(bytes: &'static [u8], light: [u8; 3], dark: [u8; 3]) -> Self {
        Self {
            light: tint(bytes, light),
            dark: tint(bytes, dark),
        }
    }

    fn widget(&self, p: Palette, size: f32) -> Widget {
        self.with_light_foreground(p.dark, size)
    }

    fn on_accent(&self, p: Palette, size: f32) -> Widget {
        self.with_light_foreground(!p.dark, size)
    }

    fn with_light_foreground(&self, light: bool, size: f32) -> Widget {
        let image = if light { &self.dark } else { &self.light };
        Image::new(image.clone())
            .width(size)
            .height(size)
            .fit(BoxFit::Contain)
            .into()
    }
}

fn tint(bytes: &'static [u8], color: [u8; 3]) -> ImageHandle {
    let source = ImageHandle::embedded(bytes).expect("icon PNG");
    let decoded = source.decoded();
    let mut pixels = decoded.pixels().to_vec();
    for pixel in pixels.as_chunks_mut::<4>().0 {
        if pixel[3] != 0 {
            pixel[..3].copy_from_slice(&color);
        }
    }
    ImageHandle::from_rgba8(decoded.width(), decoded.height(), pixels).expect("tinted icon")
}

#[derive(Clone)]
struct Icons {
    drag: ThemedIcon,
    copy: ThemedIcon,
    check: ThemedIcon,
    mic: ThemedIcon,
    stop: ThemedIcon,
    settings: ThemedIcon,
    close: ThemedIcon,
    hourglass: ThemedIcon,
    subject: ThemedIcon,
    download: ThemedIcon,
    info: ThemedIcon,
    save: ThemedIcon,
    keyboard: ThemedIcon,
    palette: ThemedIcon,
    back: ThemedIcon,
    refresh: ThemedIcon,
}

impl Icons {
    fn load() -> Self {
        macro_rules! icon {
            ($file:literal) => {
                ThemedIcon::load(include_bytes!(concat!(
                    "../assets/icons/rounded/",
                    $file,
                    ".png"
                )))
            };
        }
        Self {
            drag: icon!("drag_indicator"),
            copy: icon!("content_copy"),
            check: ThemedIcon::with_colors(
                include_bytes!("../assets/icons/rounded/check.png"),
                [39, 124, 67],
                [123, 213, 152],
            ),
            mic: icon!("mic"),
            stop: icon!("stop"),
            settings: icon!("settings"),
            close: icon!("close"),
            hourglass: icon!("hourglass_empty"),
            subject: icon!("notes"),
            download: icon!("download"),
            info: icon!("info"),
            save: icon!("save"),
            keyboard: icon!("keyboard"),
            palette: icon!("palette"),
            back: icon!("arrow_back"),
            refresh: icon!("refresh"),
        }
    }
}

fn main() {
    let state = State::new();
    let icons = Icons::load();
    let controller = RecordingController::start();
    ctrl_shortcut::start(controller.commands(), state.shortcut.clone());
    let app = create_application(state.clone(), icons, controller);
    // Visual QA uses the same success animation without changing the clipboard
    // or loading either model.
    if std::env::var_os("RAMBLER_CAPTURE").is_some()
        && std::env::var_os("RAMBLER_CAPTURE_COPY").is_some()
    {
        state.transcript.set("Copy animation preview".into());
        let motion = state.motion.clone();
        app.spawn_into(
            async {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            },
            move |result, _| {
                if result.is_ok() {
                    motion.copied();
                }
            },
        );
    }
    spawn_capture_if_requested(app.simulation());
    incular::run(app).expect("native Rambler application");
}

fn create_application(state: State, icons: Icons, controller: RecordingController) -> Application {
    let handle_slot = Rc::new(RefCell::new(None::<WindowHandle>));
    let started = Cell::new(false);
    let window_state = state.clone();
    let window_commands = controller.commands();
    let window_handle = handle_slot.clone();
    let app = Application::new_with_options(
        window_options(TOOLBAR, state.preferences.get().always_on_top),
        move |cx| {
            if !started.replace(true) {
                listen(cx, controller.clone(), window_state.clone());
            }
            build(
                cx,
                window_state.clone(),
                icons.clone(),
                window_commands.clone(),
                window_handle.clone(),
            )
        },
    )
    .expect("Rambler window");
    *handle_slot.borrow_mut() = Some(
        app.window_handle(app.primary_window())
            .expect("Rambler window handle"),
    );
    app
}

fn window_options(size: Size, always_on_top: bool) -> WindowOptions {
    WindowOptions {
        title: "Rambler".into(),
        initial_logical_size: size,
        minimum_logical_size: Some(size),
        maximum_logical_size: Some(size),
        // Retained content owns the two fixed page sizes. This also reconciles
        // late native configure events during rapid toolbar/settings changes.
        size_policy: WindowSizePolicy::Content,
        resizable: true,
        decorations: false,
        transparency_mode: TransparencyMode::Transparent,
        background_color: Color::TRANSPARENT,
        window_level: window_level(always_on_top),
        ..WindowOptions::default()
    }
}

fn window_level(always_on_top: bool) -> WindowLevel {
    if always_on_top {
        WindowLevel::AlwaysOnTop
    } else {
        WindowLevel::Normal
    }
}

fn listen(cx: &BuildContext, controller: RecordingController, state: State) {
    cx.spawn_blocking(
        move || controller.next_event().map(|event| (event, controller)),
        move |result, runtime| {
            if let Ok(Some((event, controller))) = result {
                apply_event(event, &state);
                listen_again(runtime, controller, state);
            }
        },
    );
}

fn listen_again(runtime: &mut Runtime, controller: RecordingController, state: State) {
    runtime.spawn_blocking(
        move || controller.next_event().map(|event| (event, controller)),
        move |result, runtime| {
            if let Ok(Some((event, controller))) = result {
                apply_event(event, &state);
                listen_again(runtime, controller, state);
            }
        },
    );
}

fn apply_event(event: RecordingEvent, state: &State) {
    match event {
        RecordingEvent::Phase(value) => {
            if value == Phase::Recording {
                state.motion.reset_copy();
            }
            state.phase.set(value);
        }
        RecordingEvent::Transcript(value) => {
            state.transcript.set(value);
            state.motion.reset_copy();
        }
        RecordingEvent::Error(value) => {
            state.error.set(value);
        }
    }
}

fn build(
    cx: &BuildContext,
    state: State,
    icons: Icons,
    commands: Sender<RecordingCommand>,
    handle: Rc<RefCell<Option<WindowHandle>>>,
) -> Widget {
    state.motion.bind(cx, state.phase.get());
    let preference = state.preferences.get().theme;
    let palette = match preference {
        ThemePreference::Light => Palette::light(),
        ThemePreference::Dark => Palette::dark(),
        ThemePreference::System if system_prefers_dark(cx) => Palette::dark(),
        ThemePreference::System => Palette::light(),
    };
    let window = UiWindow {
        handle,
        state,
        icons,
        commands,
    };
    let view = window.state.view.get();
    let expected = match view {
        View::Toolbar => TOOLBAR,
        View::Settings => SETTINGS,
    };
    let viewport = cx.viewport();
    let tolerance = 1.0 / cx.scale_factor() as f32;
    if (viewport.width - expected.width).abs() > tolerance
        || (viewport.height - expected.height).abs() > tolerance
    {
        // A late compositor configure may restore the preceding page's size.
        // Reconcile after this build, accepting one physical pixel of rounding.
        let reconcile = window.clone();
        cx.spawn_into(async {}, move |result, _| {
            if result.is_ok() && reconcile.state.view.get() == view {
                reconcile.show(view);
            }
        });
    }
    let page_opacity = window.state.motion.page_opacity.clone();
    let content = match view {
        View::Settings => settings(window, palette),
        View::Toolbar => toolbar(window, palette),
    };
    FadeTransition::new(page_opacity, content).into()
}

fn system_prefers_dark(cx: &BuildContext) -> bool {
    #[cfg(target_os = "linux")]
    {
        static DARK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *DARK.get_or_init(|| {
            let gtk_theme = std::env::var("GTK_THEME").unwrap_or_default();
            if gtk_theme.to_ascii_lowercase().contains("dark") {
                return true;
            }
            std::process::Command::new("gsettings")
                .args(["get", "org.gnome.desktop.interface", "color-scheme"])
                .output()
                .ok()
                .filter(|output| output.status.success())
                .is_some_and(|output| {
                    String::from_utf8_lossy(&output.stdout)
                        .to_ascii_lowercase()
                        .contains("dark")
                })
                || cx.brightness() == Brightness::Dark
        })
    }
    #[cfg(not(target_os = "linux"))]
    {
        cx.brightness() == Brightness::Dark
    }
}

fn toolbar(window: UiWindow, p: Palette) -> Widget {
    let state = window.state.clone();
    let icons = window.icons.clone();
    let commands = window.commands.clone();
    let phase = state.phase.get();
    let has_text = !state.transcript.get().is_empty();
    let copy_state = state.clone();
    let record_state = state.clone();
    let settings_window = window.clone();
    let copied = state.motion.copy_phase.get().shows_check();

    let drag: Widget = WindowDragRegion::new(
        Container::new()
            .width(ROUND_BUTTON)
            .height(ROUND_BUTTON)
            .alignment(Alignment::CENTER)
            .child(icons.drag.widget(p, TOOLBAR_ICON)),
    )
    .into();
    let copy = round_button(
        "Copy transcript",
        &state.motion,
        state.motion.copy_icon.widget(if copied {
            icons.check.widget(p, TOOLBAR_ICON)
        } else {
            icons.copy.widget(p, TOOLBAR_ICON)
        }),
        if copied { p.success_soft } else { p.soft },
        has_text && state.motion.ready.get(),
        move || {
            let text = copy_state.transcript.get();
            if text.is_empty() {
                return;
            }
            let mut clipboard = copy_state.clipboard.borrow_mut();
            let result = (|| {
                if clipboard.is_none() {
                    *clipboard = Some(arboard::Clipboard::new()?);
                }
                clipboard
                    .as_mut()
                    .expect("clipboard initialized")
                    .set_text(text)
            })();
            match result {
                Ok(()) => copy_state.motion.copied(),
                Err(error) => {
                    copy_state.motion.reset_copy();
                    copy_state.error.set(format!("Could not copy: {error}"));
                }
            };
        },
    );
    let record = round_button(
        "Record",
        &state.motion,
        state.motion.record_icon.widget(
            RotationTransition::new(
                state.motion.processing_turn.clone(),
                match phase {
                    Phase::Ready => icons.mic.on_accent(p, TOOLBAR_ICON),
                    Phase::Recording => icons.stop.on_accent(p, TOOLBAR_ICON),
                    Phase::Processing => icons.hourglass.widget(p, TOOLBAR_ICON),
                },
            )
            .into(),
        ),
        match phase {
            Phase::Ready => p.accent,
            Phase::Recording => p.danger,
            Phase::Processing => p.strong,
        },
        phase != Phase::Processing,
        move || {
            record_state.motion.reset_copy();
            record_state.error.set(String::new());
            if commands.send(RecordingCommand::Toggle).is_err() {
                record_state
                    .error
                    .set("The recording controller stopped unexpectedly.".into());
            }
        },
    );
    let record = ScaleTransition::new(state.motion.record_pulse.clone(), record).into();
    let settings = Semantics::new(round_button(
        "Settings",
        &state.motion,
        icons.settings.widget(p, TOOLBAR_ICON),
        p.surface,
        true,
        move || {
            settings_window.show(View::Settings);
        },
    ))
    .button(true)
    .label("Settings")
    .enabled(true)
    .into();
    let bar = Container::new()
        .width(TOOLBAR.width - WINDOW_INSET * 2.0)
        .height(TOOLBAR.height - WINDOW_INSET * 2.0)
        .padding(EdgeInsets::all(SPACE_SM))
        .decoration(
            BoxDecoration::new()
                .color(p.surface)
                .border(Border::new(1.0, p.border))
                .border_radius(BorderRadius::circular(
                    (TOOLBAR.height - WINDOW_INSET * 2.0) / 2.0,
                )),
        )
        .child(
            Row::new([drag, copy, record, settings])
                .spacing(SPACE_SM)
                .cross_axis_alignment(CrossAxisAlignment::Center),
        );

    Container::new()
        .width(TOOLBAR.width)
        .height(TOOLBAR.height)
        .padding(EdgeInsets::all(WINDOW_INSET))
        .child(bar)
        .into()
}

fn settings(window: UiWindow, p: Palette) -> Widget {
    let state = window.state.clone();
    let icons = window.icons.clone();
    let back_window = window.clone();
    let back = Semantics::new(round_button(
        "Back",
        &state.motion,
        icons.back.widget(p, TOOLBAR_ICON),
        p.soft,
        true,
        move || {
            back_window.show(View::Toolbar);
        },
    ))
    .button(true)
    .label("Back")
    .enabled(true)
    .into();
    let header = Container::new()
        .height(80.0)
        .padding(EdgeInsets::symmetric(SPACE_LG, 0.0))
        .child(
            Row::new([
                back,
                Expanded::new(
                    Column::new([
                        text("Settings", 19.0, p.text, FontWeight::W700),
                        text(
                            "Rambler runs entirely on this computer",
                            12.0,
                            p.muted,
                            FontWeight::W400,
                        ),
                    ])
                    .spacing(SPACE_XS)
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Start),
                )
                .into(),
                action_button(
                    icons.close.widget(p, ACTION_ICON),
                    "Quit",
                    p,
                    false,
                    true,
                    &state.motion,
                    {
                        let window = window.clone();
                        move || {
                            if let Some(handle) = window.handle.borrow().as_ref()
                                && let Err(error) = handle.close()
                            {
                                window
                                    .state
                                    .error
                                    .set(format!("Could not close Rambler: {error}"));
                            }
                        }
                    },
                ),
            ])
            .spacing(SPACE_MD)
            .cross_axis_alignment(CrossAxisAlignment::Center),
        );

    let save_state = state.clone();
    let vocabulary = state.vocabulary.clone();
    let vocabulary_section = section(
        icons.subject.widget(p, ACTION_ICON),
        "Vocabulary",
        "One canonical name or phrase per line. Changes apply to the next recording.",
        vec![
            TextArea::new(state.vocabulary.clone())
                .size(Size::new(0.0, 96.0))
                .style(TextFieldStyle {
                    background: Some(p.soft),
                    foreground: Some(p.text),
                    placeholder_color: Some(p.muted),
                    border: Some(Border::new(1.0, p.border)),
                    border_focused: Some(Border::new(1.0, p.accent)),
                    border_radius: Some(SPACE_MD),
                    padding: Some(EdgeInsets::all(SPACE_MD)),
                })
                .placeholder("Fanisus\nRambler Desktop\nQwen3-ASR")
                .into(),
            action_line(
                state.vocabulary_status.get(),
                action_button(
                    if state.motion.confirmed("Save vocabulary") {
                        icons.check.widget(p, ACTION_ICON)
                    } else {
                        icons.save.on_accent(p, ACTION_ICON)
                    },
                    "Save vocabulary",
                    p,
                    true,
                    true,
                    &state.motion,
                    move || {
                        match rambler::models::save_vocabulary_source(&vocabulary.text()) {
                            Ok(()) => {
                                save_state.motion.confirm("Save vocabulary");
                                save_state.vocabulary_status.set("Saved".into())
                            }
                            Err(error) => save_state.vocabulary_status.set(format!("{error:#}")),
                        };
                    },
                ),
                p,
            ),
        ],
        p,
    );

    let model = state.model_status.get();
    let download_action_state = state.download.state();
    let downloading = matches!(download_action_state, ActionState::Loading);
    let download_message = match &download_action_state {
        ActionState::Idle => state.download_status.get(),
        ActionState::Loading => "Downloading missing model files…".to_string(),
        ActionState::Ready(message) => message.clone(),
        ActionState::Error(error) => format!("{error:?}"),
    };
    let recheck_state = state.clone();
    let download_state = state.clone();
    let model_section = section(
        icons.download.widget(p, ACTION_ICON),
        "Local models",
        "Downloads only the assets required by the model names compiled into Rambler.",
        vec![
            setting_row(
                "Qwen3-ASR + S1-mini",
                format!(
                    "{} of {} required files",
                    model.installed_files, model.total_files
                ),
                badge(
                    if model.installed() {
                        "Installed"
                    } else {
                        "Missing files"
                    },
                    model.installed(),
                    p,
                ),
                p,
            ),
            action_line(
                download_message,
                Row::new([
                    action_button(
                        if state.motion.confirmed("Recheck") {
                            icons.check.widget(p, ACTION_ICON)
                        } else {
                            icons.refresh.widget(p, ACTION_ICON)
                        },
                        "Recheck",
                        p,
                        false,
                        !downloading,
                        &state.motion,
                        move || {
                            match model_download::status() {
                                Ok(status) => {
                                    recheck_state.model_status.set(status);
                                    recheck_state.motion.confirm("Recheck");
                                    recheck_state.download_status.set("Checked".into());
                                }
                                Err(error) => {
                                    recheck_state.download_status.set(format!("{error:#}"));
                                }
                            };
                        },
                    ),
                    action_button(
                        if downloading {
                            icons.hourglass.on_accent(p, ACTION_ICON)
                        } else {
                            icons.download.on_accent(p, ACTION_ICON)
                        },
                        "Download missing",
                        p,
                        true,
                        !downloading && !model.installed(),
                        &state.motion,
                        move || {
                            if model.installed() {
                                return;
                            }
                            let _ = download_state.download.dispatch(());
                        },
                    ),
                ])
                .spacing(SPACE_SM)
                .main_axis_size(MainAxisSize::Min)
                .into(),
                p,
            ),
        ],
        p,
    );

    let current = state.preferences.get();
    let always_state = state.clone();
    let always_window = window.clone();
    let appearance = section(
        icons.palette.widget(p, ACTION_ICON),
        "Appearance",
        "Choose how Rambler looks and behaves above other windows.",
        vec![
            setting_row(
                "Theme",
                "Follow the system or choose a palette",
                Widget::from(
                    Row::new([
                        theme_button("System", ThemePreference::System, &state, p),
                        theme_button("Light", ThemePreference::Light, &state, p),
                        theme_button("Dark", ThemePreference::Dark, &state, p),
                    ])
                    .spacing(SPACE_XS)
                    .main_axis_size(MainAxisSize::Min),
                ),
                p,
            ),
            setting_row(
                "Always on top",
                "Keep the compact toolbar above other windows",
                Switch::new(current.always_on_top)
                    .active_track_color(p.accent)
                    .on_changed(move |value| {
                        if let Some(handle) = always_window.handle.borrow().as_ref()
                            && let Err(error) = handle.set_window_level(window_level(value))
                        {
                            always_state
                                .error
                                .set(format!("Could not change window level: {error}"));
                            return;
                        }
                        let mut next = always_state.preferences.get();
                        next.always_on_top = value;
                        persist(&always_state, next);
                    })
                    .into(),
                p,
            ),
        ],
        p,
    );

    let shortcut_state = state.clone();
    let shortcut = section(
        icons.keyboard.widget(p, ACTION_ICON),
        "Shortcut",
        "Tap Ctrl once, then press Ctrl again and hold while speaking. Release to submit.",
        vec![setting_row(
            "Global Ctrl shortcut",
            "Ctrl+C and other normal shortcuts still pass through",
            Switch::new(current.shortcut_enabled)
                .active_track_color(p.accent)
                .on_changed(move |value| {
                    let mut next = shortcut_state.preferences.get();
                    next.shortcut_enabled = value;
                    persist(&shortcut_state, next);
                })
                .into(),
            p,
        )],
        p,
    );
    let about = section(
        icons.info.widget(p, ACTION_ICON),
        "About",
        "A private, CPU-only dictation pipeline with no hosted inference.",
        vec![
            about_row("Rambler", concat!("v", env!("CARGO_PKG_VERSION")), p),
            about_row("Pipeline", "Audio → ASR → Cleaner → Output", p),
            about_row("Models", "Qwen3-ASR + S1-mini by Superwhisper", p),
        ],
        p,
    );
    let mut sections = vec![
        vocabulary_section,
        model_section,
        appearance,
        shortcut,
        about,
    ];
    if !state.error.get().is_empty() {
        sections.insert(0, text(state.error.get(), 12.0, p.danger, FontWeight::W500));
    }
    if !state.settings_status.get().is_empty() {
        sections.push(text(
            state.settings_status.get(),
            11.0,
            p.success,
            FontWeight::W500,
        ));
    }
    let content = SingleChildScrollView::new(
        Column::new(sections)
            .spacing(SPACE_LG)
            .cross_axis_alignment(CrossAxisAlignment::Stretch),
    )
    .padding(EdgeInsets {
        left: SPACE_LG,
        right: SPACE_LG,
        top: 0.0,
        bottom: SPACE_LG,
    });

    Container::new()
        .width(SETTINGS.width)
        .height(SETTINGS.height)
        .padding(EdgeInsets::all(WINDOW_INSET))
        .child(
            Container::new()
                .decoration(
                    BoxDecoration::new()
                        .color(p.surface)
                        .border(Border::new(1.0, p.border))
                        .border_radius(BorderRadius::circular(24.0)),
                )
                .clip_behavior(Clip::AntiAlias)
                .child(Column::new([
                    WindowDragRegion::new(header).into(),
                    Widget::from(Expanded::new(content)),
                ])),
        )
        .into()
}

fn round_button(
    label: &'static str,
    motion: &Motion,
    child: Widget,
    background: Color,
    enabled: bool,
    tap: impl Fn() + 'static,
) -> Widget {
    let visual = Container::new()
        .width(ROUND_BUTTON)
        .height(ROUND_BUTTON)
        .alignment(Alignment::CENTER)
        .decoration(
            BoxDecoration::new()
                .color(background)
                .border_radius(BorderRadius::circular(ROUND_BUTTON / 2.0)),
        )
        .child(if enabled {
            child
        } else {
            Opacity::new(0.45, child).into()
        });
    motion.interactive(label, visual.into(), enabled, tap)
}

fn section(
    icon: Widget,
    title: &'static str,
    description: &'static str,
    body: Vec<Widget>,
    p: Palette,
) -> Widget {
    Container::new()
        .padding(EdgeInsets::all(SPACE_LG))
        .decoration(
            BoxDecoration::new()
                .color(p.raised)
                .border(Border::new(1.0, p.border))
                .border_radius(BorderRadius::circular(16.0)),
        )
        .child(
            Column::new([
                Widget::from(
                    Row::new([
                        Widget::from(
                            Container::new()
                                .width(ACTION_HEIGHT)
                                .height(ACTION_HEIGHT)
                                .alignment(Alignment::CENTER)
                                .decoration(
                                    BoxDecoration::new()
                                        .color(p.soft)
                                        .border_radius(BorderRadius::circular(SPACE_MD)),
                                )
                                .child(icon),
                        ),
                        Widget::from(Expanded::new(
                            Column::new([
                                text(title, 14.0, p.text, FontWeight::W600),
                                text(description, 11.0, p.muted, FontWeight::W400),
                            ])
                            .spacing(SPACE_XS)
                            .cross_axis_alignment(CrossAxisAlignment::Start),
                        )),
                    ])
                    .spacing(SPACE_MD)
                    .cross_axis_alignment(CrossAxisAlignment::Center),
                ),
                Widget::from(
                    Column::new(body)
                        .spacing(SPACE_LG)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch),
                ),
            ])
            .spacing(SPACE_LG)
            .cross_axis_alignment(CrossAxisAlignment::Stretch),
        )
        .into()
}

fn setting_row(
    title: impl Into<String>,
    description: impl Into<String>,
    control: Widget,
    p: Palette,
) -> Widget {
    Row::new([
        Expanded::new(
            Column::new([
                text(title, 12.0, p.text, FontWeight::W600),
                text(description, 10.0, p.muted, FontWeight::W400),
            ])
            .spacing(SPACE_XS)
            .cross_axis_alignment(CrossAxisAlignment::Start),
        )
        .into(),
        control,
    ])
    .spacing(SPACE_LG)
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .into()
}

fn theme_button(label: &'static str, theme: ThemePreference, state: &State, p: Palette) -> Widget {
    let selected = state.preferences.get().theme == theme;
    let next_state = state.clone();
    state.motion.interactive(
        label,
        Container::new()
            .height(ACTION_HEIGHT)
            .padding(EdgeInsets::symmetric(SPACE_MD, 0.0))
            .alignment(Alignment::CENTER)
            .decoration(
                BoxDecoration::new()
                    .color(if selected { p.accent_soft } else { p.soft })
                    .border_radius(BorderRadius::circular(SPACE_MD)),
            )
            .child(text(
                label,
                11.0,
                if selected { p.accent } else { p.text },
                FontWeight::W500,
            ))
            .into(),
        true,
        move || {
            let mut next = next_state.preferences.get();
            next.theme = theme;
            persist(&next_state, next);
        },
    )
}

fn persist(state: &State, next: Preferences) {
    state.shortcut.set_enabled(next.shortcut_enabled);
    match next.save() {
        Ok(()) => state.settings_status.set("Saved".into()),
        Err(error) => state
            .settings_status
            .set(format!("Could not save: {error:#}")),
    };
    state.preferences.set(next);
}

fn action_line(status: String, action: Widget, p: Palette) -> Widget {
    Row::new([
        Expanded::new(text(status, 11.0, p.muted, FontWeight::W400)).into(),
        action,
    ])
    .spacing(SPACE_SM)
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .main_axis_alignment(MainAxisAlignment::SpaceBetween)
    .into()
}

fn action_button(
    icon: Widget,
    label: &'static str,
    p: Palette,
    primary: bool,
    enabled: bool,
    motion: &Motion,
    tap: impl Fn() + 'static,
) -> Widget {
    let confirmed = motion.confirmed(label);
    let visual = Container::new()
        .height(ACTION_HEIGHT)
        .padding(EdgeInsets::symmetric(SPACE_MD, 0.0))
        .decoration(
            BoxDecoration::new()
                .color(if confirmed {
                    p.success_soft
                } else if primary {
                    p.accent
                } else {
                    p.soft
                })
                .border_radius(BorderRadius::circular(SPACE_MD)),
        )
        .alignment(Alignment::CENTER)
        .child(
            Row::new([
                icon,
                text(
                    label,
                    11.0,
                    if confirmed {
                        p.success
                    } else if primary {
                        if p.dark {
                            Palette::light().text
                        } else {
                            Color::WHITE
                        }
                    } else {
                        p.text
                    },
                    FontWeight::W600,
                ),
            ])
            .spacing(SPACE_SM)
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .main_axis_size(MainAxisSize::Min),
        );
    let visual: Widget = visual.into();
    motion.interactive(
        label,
        if enabled {
            visual
        } else {
            Opacity::new(0.45, visual).into()
        },
        enabled,
        tap,
    )
}

fn badge(label: &'static str, installed: bool, p: Palette) -> Widget {
    Container::new()
        .padding(EdgeInsets::symmetric(SPACE_SM, SPACE_XS))
        .decoration(
            BoxDecoration::new()
                .color(if installed { p.accent_soft } else { p.strong })
                .border_radius(BorderRadius::circular(12.0)),
        )
        .child(text(
            label,
            10.0,
            if installed { p.success } else { p.muted },
            FontWeight::W600,
        ))
        .into()
}

fn about_row(label: &'static str, value: &'static str, p: Palette) -> Widget {
    Row::new([
        Expanded::new(text(label, 11.0, p.muted, FontWeight::W400)).into(),
        text(value, 11.0, p.text, FontWeight::W600),
    ])
    .spacing(SPACE_MD)
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .into()
}

fn text(value: impl Into<String>, size: f32, color: Color, weight: FontWeight) -> Widget {
    Text::new(value)
        .style(
            TextStyle::new()
                .font_size(size)
                .font_weight(weight)
                .color(color),
        )
        .into()
}

impl UiWindow {
    fn show(&self, view: View) {
        let size = match view {
            View::Toolbar => TOOLBAR,
            View::Settings => SETTINGS,
        };
        let limits = incular::platform::LogicalSizeLimits::new(Some(size), Some(size))
            .expect("fixed page sizes are valid");
        let handle = self.handle.borrow();
        let Some(handle) = handle.as_ref() else {
            self.state
                .error
                .set("Rambler window is unavailable".to_string());
            return;
        };
        match handle.set_logical_size_limits(limits) {
            Ok(()) => {
                if self.state.view.get() != view {
                    self.state.motion.page_enter();
                }
                self.state.view.set(view);
            }
            Err(error) => {
                self.state
                    .error
                    .set(format!("Could not resize Rambler: {error}"));
            }
        }
    }
}

fn spawn_capture_if_requested(simulation: incular::testing::Simulation) {
    let Ok(path) = std::env::var("RAMBLER_CAPTURE") else {
        return;
    };
    let open_settings = std::env::var_os("RAMBLER_CAPTURE_SETTINGS").is_some();
    let roundtrip = std::env::var_os("RAMBLER_CAPTURE_ROUNDTRIP").is_some();
    let exit = std::env::var_os("RAMBLER_CAPTURE_EXIT").is_some();
    let sequence = std::env::var("RAMBLER_CAPTURE_SEQUENCE").ok();
    let scroll = std::env::var("RAMBLER_CAPTURE_SCROLL")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.0);
    std::thread::spawn(move || {
        let result = (|| -> anyhow::Result<()> {
            let initial = simulation.capture()?;
            if let Some(directory) = &sequence {
                std::fs::create_dir_all(directory)?;
                let until = std::time::Instant::now() + std::time::Duration::from_millis(3600);
                let mut index = 0;
                while std::time::Instant::now() < until {
                    let frame = simulation.capture()?;
                    image::save_buffer(
                        std::path::Path::new(directory).join(format!("{index:03}.png")),
                        frame.pixels(),
                        frame.width(),
                        frame.height(),
                        image::ColorType::Rgba8,
                    )?;
                    index += 1;
                    std::thread::sleep(std::time::Duration::from_millis(40));
                }
            }
            let scale = initial.width() as f32 / TOOLBAR.width;
            if roundtrip {
                for _ in 0..5 {
                    simulation.click("Settings")?;
                    capture_at_size(&simulation, SETTINGS, scale)?;
                    simulation.click("Back")?;
                    capture_at_size(&simulation, TOOLBAR, scale)?;
                }
            }
            let frame = if open_settings {
                simulation.click("Settings")?;
                capture_at_size(&simulation, SETTINGS, scale)?;
                if scroll != 0.0 {
                    simulation
                        .move_mouse_to(Offset::new(SETTINGS.width / 2.0, SETTINGS.height / 2.0))?;
                    simulation.scroll(Offset::new(0.0, scroll))?;
                }
                capture_at_size(&simulation, SETTINGS, scale)?
            } else {
                capture_at_size(&simulation, TOOLBAR, scale)?
            };
            image::save_buffer(
                path,
                frame.pixels(),
                frame.width(),
                frame.height(),
                image::ColorType::Rgba8,
            )?;
            Ok(())
        })();
        if let Err(error) = &result {
            eprintln!("Rambler capture failed: {error:#}");
        }
        if exit {
            std::process::exit(i32::from(result.is_err()));
        }
    });
}

fn capture_at_size(
    simulation: &incular::testing::Simulation,
    logical: Size,
    scale: f32,
) -> anyhow::Result<incular::testing::Screenshot> {
    // Native resize completion is asynchronous. Inspect actual frames instead
    // of relying on a fixed number of redraws or a timing-dependent sleep.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    // The scale estimate comes from a small, rounded physical toolbar, and
    // content sizing can round outward once more on fractional-scale desktops.
    let tolerance = (3.0 * scale).max(4.0);
    let mut settled = false;
    loop {
        let frame = simulation.capture()?;
        let matches = (frame.width() as f32 - logical.width * scale).abs() <= tolerance
            && (frame.height() as f32 - logical.height * scale).abs() <= tolerance;
        if matches && settled {
            return Ok(frame);
        }
        settled = matches;
        if std::time::Instant::now() >= deadline {
            anyhow::bail!(
                "window did not resize to {} × {} at scale {scale}; last capture was {} × {}",
                logical.width,
                logical.height,
                frame.width(),
                frame.height(),
            );
        }
    }
}
