use std::sync::{Arc, Mutex, mpsc};

use rambler::audio;
use rambler::models::{InferenceJob, start_model_worker};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Phase {
    Ready,
    Recording,
    Processing,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum RecordingCommand {
    Toggle,
    StartHold,
    StopHold,
}

#[derive(Debug)]
pub(crate) enum RecordingEvent {
    Phase(Phase),
    Transcript(String),
    Error(String),
}

#[derive(Clone)]
pub(crate) struct RecordingController {
    commands: mpsc::Sender<RecordingCommand>,
    events: Arc<Mutex<mpsc::Receiver<RecordingEvent>>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum RecordingMode {
    Hold,
    Toggle,
}

impl RecordingController {
    pub(crate) fn start() -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();

        std::thread::Builder::new()
            .name("recording-controller".to_string())
            .spawn(move || run(command_rx, event_tx))
            .expect("failed to start recording controller");

        Self {
            commands: command_tx,
            events: Arc::new(Mutex::new(event_rx)),
        }
    }

    pub(crate) fn commands(&self) -> mpsc::Sender<RecordingCommand> {
        self.commands.clone()
    }

    pub(crate) fn next_event(&self) -> Option<RecordingEvent> {
        self.events.lock().ok()?.recv().ok()
    }
}

fn run(commands: mpsc::Receiver<RecordingCommand>, events: mpsc::Sender<RecordingEvent>) {
    let mut model_worker = None;
    let mut mode = None;

    while let Ok(command) = commands.recv() {
        match command {
            RecordingCommand::Toggle => match mode {
                None => start_recording(RecordingMode::Toggle, &mut mode, &events),
                Some(_) => {
                    finish_recording(&mut model_worker, &mut mode, &events);
                    discard_commands_queued_during_inference(&commands);
                }
            },
            RecordingCommand::StartHold if mode.is_none() => {
                start_recording(RecordingMode::Hold, &mut mode, &events);
            }
            RecordingCommand::StopHold if mode == Some(RecordingMode::Hold) => {
                finish_recording(&mut model_worker, &mut mode, &events);
                discard_commands_queued_during_inference(&commands);
            }
            RecordingCommand::StartHold | RecordingCommand::StopHold => {}
        }
    }
}

fn discard_commands_queued_during_inference(commands: &mpsc::Receiver<RecordingCommand>) {
    while commands.try_recv().is_ok() {}
}

fn start_recording(
    recording_mode: RecordingMode,
    mode: &mut Option<RecordingMode>,
    events: &mpsc::Sender<RecordingEvent>,
) {
    match audio::start_recording() {
        Ok(()) => {
            *mode = Some(recording_mode);
            send_event(events, RecordingEvent::Phase(Phase::Recording));
        }
        Err(message) => send_error(events, message),
    }
}

fn finish_recording(
    model_worker: &mut Option<mpsc::Sender<InferenceJob>>,
    mode: &mut Option<RecordingMode>,
    events: &mpsc::Sender<RecordingEvent>,
) {
    *mode = None;

    let samples = match audio::stop_recording() {
        Ok(samples) if samples.len() >= 1_600 => samples,
        Ok(_) => {
            send_error(events, "That recording was too short.".to_string());
            send_event(events, RecordingEvent::Phase(Phase::Ready));
            return;
        }
        Err(message) => {
            send_error(events, message);
            send_event(events, RecordingEvent::Phase(Phase::Ready));
            return;
        }
    };

    send_event(events, RecordingEvent::Phase(Phase::Processing));
    let (reply_tx, reply_rx) = mpsc::channel();
    if model_worker
        .get_or_insert_with(start_model_worker)
        .send(InferenceJob {
            samples,
            reply: reply_tx,
        })
        .is_err()
    {
        send_error(events, "The model worker stopped unexpectedly.".to_string());
        send_event(events, RecordingEvent::Phase(Phase::Ready));
        return;
    }

    match reply_rx.recv() {
        Ok(Ok(text)) => send_event(events, RecordingEvent::Transcript(text)),
        Ok(Err(message)) => send_error(events, message),
        Err(_) => send_error(events, "The model worker did not respond.".to_string()),
    }
    send_event(events, RecordingEvent::Phase(Phase::Ready));
}

fn send_error(events: &mpsc::Sender<RecordingEvent>, message: String) {
    eprintln!("{message}");
    send_event(events, RecordingEvent::Error(message));
}

fn send_event(events: &mpsc::Sender<RecordingEvent>, event: RecordingEvent) {
    let _ = events.send(event);
}
