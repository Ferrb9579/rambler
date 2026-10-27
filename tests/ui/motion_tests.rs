use super::*;
use crate::motion::CopyPhase;

fn until(app: &mut Application, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(4);
    while !condition() {
        assert!(
            Instant::now() < deadline,
            "motion did not reach its expected state"
        );
        app.process_runtime_work();
        frame(app, TOOLBAR);
        std::thread::park_timeout(Duration::from_millis(2));
    }
}

fn ready() -> (State, Application) {
    let state = State::new();
    let mut app = create_application(state.clone(), Icons::load(), RecordingController::start());
    until(&mut app, || state.motion.ready.get());
    (state, app)
}

#[test]
fn copy_blurs_swaps_to_success_and_returns_without_changing_window_size() {
    let (state, mut app) = ready();
    state.motion.copied();
    assert_eq!(state.motion.copy_phase.get(), CopyPhase::Leaving);
    until(&mut app, || {
        state.motion.copy_phase.get() == CopyPhase::Success
    });
    assert!(state.motion.copy_phase.get().shows_check());
    until(&mut app, || {
        state.motion.copy_phase.get() == CopyPhase::Idle
    });
    assert_eq!(app.active_window_ids().len(), 1);
    assert!(app.take_native_window_commands().iter().all(|command| !matches!(
        command, NativeWindowCommand::Operate(command)
            if matches!(command.operation, WindowOperation::SetLogicalSize(size) if size != TOOLBAR)
    )));
}

#[test]
fn new_transcript_cancels_pending_copy_feedback() {
    let (state, mut app) = ready();
    state.motion.copied();
    // A second copy cancels the previous exit/success timers.
    state.motion.copied();
    apply_event(
        RecordingEvent::Transcript("A newer transcript".into()),
        &state,
    );
    let deadline = Instant::now() + Duration::from_millis(250);
    while Instant::now() < deadline {
        app.process_runtime_work();
        frame(&mut app, TOOLBAR);
        std::thread::park_timeout(Duration::from_millis(2));
    }
    assert_eq!(state.motion.copy_phase.get(), CopyPhase::Idle);
}

#[test]
fn reduced_motion_keeps_success_feedback_but_disables_scale_and_spin() {
    let (state, mut app) = ready();
    app.set_window_environment(
        app.primary_window(),
        RuntimeEnvironment {
            viewport: TOOLBAR,
            reduced_motion: true,
            ..RuntimeEnvironment::default()
        },
    );
    state.phase.set(Phase::Recording);
    app.run_window_frame_at(
        app.primary_window(),
        Constraints::tight(TOOLBAR),
        Instant::now(),
    )
    .unwrap();
    state.motion.copied();
    // Do not replace the environment with simulated native metrics in this test.
    let deadline = Instant::now() + Duration::from_secs(2);
    while state.motion.copy_phase.get() != CopyPhase::Success {
        assert!(Instant::now() < deadline);
        app.process_runtime_work();
        std::thread::park_timeout(Duration::from_millis(2));
    }
    assert_eq!(state.motion.copy_icon.scale.scale(), 1.0);
    assert_eq!(state.motion.record_pulse.scale(), 1.0);
    assert_eq!(state.motion.processing_turn.radians(), 0.0);
    app.close_window(app.primary_window());
    assert!(app.should_exit());
}

#[test]
fn settings_confirmations_expire() {
    let (state, mut app) = ready();
    state.motion.confirm("Save vocabulary");
    assert!(state.motion.confirmed("Save vocabulary"));
    until(&mut app, || !state.motion.confirmed("Save vocabulary"));
}

#[test]
fn recording_and_processing_motion_stop_when_ready() {
    let (state, mut app) = ready();
    for phase in [Phase::Recording, Phase::Processing] {
        apply_event(RecordingEvent::Phase(phase), &state);
        frame(&mut app, TOOLBAR);
        app.run_window_frame_at(
            app.primary_window(),
            Constraints::tight(TOOLBAR),
            Instant::now() + Duration::from_millis(350),
        )
        .unwrap();
        if phase == Phase::Recording {
            assert!(state.motion.record_pulse.scale() > 1.0);
        } else {
            assert!(state.motion.processing_turn.radians() > 0.0);
        }
        apply_event(RecordingEvent::Phase(Phase::Ready), &state);
        frame(&mut app, TOOLBAR);
        assert_eq!(state.motion.record_pulse.scale(), 1.0);
        assert_eq!(state.motion.processing_turn.radians(), 0.0);
    }
}
