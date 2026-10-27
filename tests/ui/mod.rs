use super::*;
use incular::platform::{PhysicalSize, PlatformEvent, WindowMetrics};
use std::time::{Duration, Instant};

mod motion_tests;

fn click(app: &mut Application, label: &'static str) {
    let simulation = app.simulation();
    let worker = std::thread::spawn(move || simulation.click(label));
    let deadline = Instant::now() + Duration::from_secs(5);
    while !worker.is_finished() {
        assert!(Instant::now() < deadline, "simulation click timed out");
        app.process_simulation_requests();
        std::thread::park_timeout(Duration::from_millis(1));
    }
    worker.join().unwrap().unwrap();
}

fn metrics(app: &mut Application, size: Size) {
    app.handle_window_event(WindowEvent::platform(
        app.primary_window(),
        PlatformEvent::Metrics(WindowMetrics::new(
            PhysicalSize::new(size.width as u32, size.height as u32),
            1.0,
        )),
    ));
}

fn frame(app: &mut Application, size: Size) {
    // The desktop host applies queued window options before rendering a frame.
    app.take_native_window_commands();
    metrics(app, size);
    app.run_window_frame_at(
        app.primary_window(),
        Constraints::tight(size),
        Instant::now(),
    )
    .expect("UI frame");
}

#[test]
fn late_native_configure_is_reconciled_with_settings_content() {
    let state = State::new();
    let mut app = create_application(state.clone(), Icons::load(), RecordingController::start());
    frame(&mut app, TOOLBAR);
    click(&mut app, "Settings");
    frame(&mut app, TOOLBAR);
    app.take_native_window_commands();
    // Native resize acknowledgment can be followed by an older configure
    // before a frame is presented at the new size.
    metrics(&mut app, SETTINGS);
    app.process_runtime_work();
    metrics(&mut app, TOOLBAR);
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app.process_runtime_work();
        frame(&mut app, TOOLBAR);
        if app.take_native_window_commands().iter().any(|command| {
            matches!(
                command, NativeWindowCommand::Operate(command)
                    if command.operation == WindowOperation::SetLogicalSize(SETTINGS)
            )
        }) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "late configure was not reconciled"
        );
        std::thread::park_timeout(Duration::from_millis(1));
    }
    assert!(state.view.get() == View::Settings);
}

#[test]
fn settings_roundtrip_keeps_one_window_and_closes_cleanly() {
    let state = State::new();
    let mut app = create_application(state.clone(), Icons::load(), RecordingController::start());
    assert_eq!(app.active_window_ids(), [app.primary_window()]);
    frame(&mut app, TOOLBAR);

    for _ in 0..5 {
        click(&mut app, "Settings");
        assert!(state.view.get() == View::Settings);
        frame(&mut app, TOOLBAR);
        assert!(
            app.take_native_window_commands()
                .iter()
                .any(|command| matches!(
                    command, NativeWindowCommand::Operate(command)
                        if command.operation == WindowOperation::SetLogicalSize(SETTINGS)
                ))
        );
        frame(&mut app, SETTINGS);
        click(&mut app, "Back");
        assert!(state.view.get() == View::Toolbar);
        frame(&mut app, SETTINGS);
        assert!(
            app.take_native_window_commands()
                .iter()
                .any(|command| matches!(
                    command, NativeWindowCommand::Operate(command)
                        if command.operation == WindowOperation::SetLogicalSize(TOOLBAR)
                ))
        );
        frame(&mut app, TOOLBAR);
        assert_eq!(app.active_window_ids().len(), 1);
    }

    // Removing the bootstrap window lets the normal last-window policy apply.
    assert!(app.close_window(app.primary_window()));
    assert!(app.active_window_ids().is_empty());
    assert!(app.should_exit());
}

#[test]
fn drag_handle_requests_native_window_movement() {
    let mut app = create_application(State::new(), Icons::load(), RecordingController::start());
    frame(&mut app, TOOLBAR);
    app.take_native_window_commands();
    let simulation = app.simulation();
    let worker = std::thread::spawn(move || simulation.click_at(Offset::new(24.0, 27.0)));
    let deadline = Instant::now() + Duration::from_secs(5);
    while !worker.is_finished() {
        assert!(Instant::now() < deadline, "drag click timed out");
        app.process_simulation_requests();
        std::thread::park_timeout(Duration::from_millis(1));
    }
    worker.join().unwrap().unwrap();
    assert!(
        app.take_native_window_commands()
            .iter()
            .any(|command| matches!(
                command, NativeWindowCommand::Operate(command)
                    if command.operation == WindowOperation::BeginMoveDrag
            ))
    );
}
