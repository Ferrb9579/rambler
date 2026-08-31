use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::controller::RecordingCommand;
use crate::preferences::ShortcutControl;

const DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(350);
const HOLD_THRESHOLD: Duration = Duration::from_millis(250);
const POLL_INTERVAL: Duration = Duration::from_millis(8);

pub(crate) fn start(commands: mpsc::Sender<RecordingCommand>, control: ShortcutControl) {
    #[cfg(target_os = "linux")]
    std::thread::Builder::new()
        .name("ctrl-shortcut".to_string())
        .spawn(move || linux::listen(commands, control))
        .expect("failed to start Ctrl shortcut listener");

    #[cfg(not(target_os = "linux"))]
    let _ = (commands, control);
}

#[derive(Clone, Copy)]
enum Input {
    CtrlPressed,
    CtrlReleased,
    OtherKeyPressed,
    Tick,
}

#[derive(Default)]
struct Gesture {
    ctrl_down: bool,
    hold_started: bool,
    chorded: bool,
    second_press: bool,
    pressed_at: Option<Instant>,
    last_tap_at: Option<Instant>,
}

impl Gesture {
    fn update(&mut self, input: Input, now: Instant) -> Option<RecordingCommand> {
        match input {
            Input::CtrlPressed if !self.ctrl_down => {
                self.ctrl_down = true;
                self.hold_started = false;
                self.chorded = false;
                self.pressed_at = Some(now);
                self.second_press = self
                    .last_tap_at
                    .is_some_and(|last_tap| now.duration_since(last_tap) <= DOUBLE_TAP_WINDOW);

                if self.second_press {
                    self.last_tap_at = None;
                }
                None
            }
            Input::CtrlReleased if self.ctrl_down => {
                self.ctrl_down = false;

                let action = self.hold_started.then_some(RecordingCommand::StopHold);
                let was_quick_tap = self
                    .pressed_at
                    .is_some_and(|pressed| now.duration_since(pressed) < HOLD_THRESHOLD);
                if !self.hold_started && !self.chorded && !self.second_press && was_quick_tap {
                    self.last_tap_at = Some(now);
                }
                self.hold_started = false;
                self.chorded = false;
                self.second_press = false;
                self.pressed_at = None;
                action
            }
            Input::OtherKeyPressed if self.ctrl_down => {
                self.chorded = true;
                self.last_tap_at = None;
                if self.hold_started {
                    self.hold_started = false;
                    Some(RecordingCommand::StopHold)
                } else {
                    None
                }
            }
            Input::Tick
                if self.ctrl_down
                    && !self.hold_started
                    && !self.chorded
                    && self.second_press
                    && self
                        .pressed_at
                        .is_some_and(|pressed| now.duration_since(pressed) >= HOLD_THRESHOLD) =>
            {
                self.hold_started = true;
                self.last_tap_at = None;
                Some(RecordingCommand::StartHold)
            }
            _ => None,
        }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use evdev::{Device, EventSummary, KeyCode, enumerate};

    use super::*;

    pub(super) fn listen(commands: mpsc::Sender<RecordingCommand>, control: ShortcutControl) {
        let mut keyboards: Vec<Device> = enumerate()
            .filter_map(|(_, device)| {
                let keys = device.supported_keys()?;
                (keys.contains(KeyCode::KEY_LEFTCTRL) || keys.contains(KeyCode::KEY_RIGHTCTRL))
                    .then_some(device)
            })
            .collect();

        if keyboards.is_empty() {
            eprintln!("Ctrl shortcut unavailable: Rambler cannot read any keyboard in /dev/input.");
            return;
        }

        for keyboard in &keyboards {
            if let Err(error) = keyboard.set_nonblocking(true) {
                eprintln!("Ctrl shortcut could not configure a keyboard: {error}");
            }
        }

        let mut gesture = Gesture::default();
        loop {
            if !control.is_enabled() {
                if gesture.hold_started {
                    let _ = commands.send(RecordingCommand::StopHold);
                }
                gesture = Gesture::default();
                std::thread::sleep(POLL_INTERVAL);
                continue;
            }
            for keyboard in &mut keyboards {
                match keyboard.fetch_events() {
                    Ok(events) => {
                        for event in events {
                            let EventSummary::Key(_, key, value) = event.destructure() else {
                                continue;
                            };

                            let is_ctrl =
                                matches!(key, KeyCode::KEY_LEFTCTRL | KeyCode::KEY_RIGHTCTRL);
                            let input = match (is_ctrl, value) {
                                (true, 1) => Some(Input::CtrlPressed),
                                (true, 0) => Some(Input::CtrlReleased),
                                (false, 1) => Some(Input::OtherKeyPressed),
                                _ => None,
                            };

                            if let Some(action) =
                                input.and_then(|input| gesture.update(input, Instant::now()))
                            {
                                let _ = commands.send(action);
                            }
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(error) => eprintln!("Ctrl shortcut keyboard read failed: {error}"),
                }
            }

            if let Some(action) = gesture.update(Input::Tick, Instant::now()) {
                let _ = commands.send(action);
            }
            std::thread::sleep(POLL_INTERVAL);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_tap_and_hold_records_until_release() {
        let start = Instant::now();
        let mut gesture = Gesture::default();
        assert_eq!(gesture.update(Input::CtrlPressed, start), None);
        assert_eq!(
            gesture.update(Input::CtrlReleased, start + Duration::from_millis(40)),
            None
        );
        assert_eq!(
            gesture.update(Input::CtrlPressed, start + Duration::from_millis(180)),
            None
        );
        assert_eq!(
            gesture.update(Input::Tick, start + Duration::from_millis(430)),
            Some(RecordingCommand::StartHold)
        );
        assert_eq!(
            gesture.update(Input::CtrlReleased, start + Duration::from_millis(600)),
            Some(RecordingCommand::StopHold)
        );
    }

    #[test]
    fn single_hold_does_nothing() {
        let start = Instant::now();
        let mut gesture = Gesture::default();
        assert_eq!(gesture.update(Input::CtrlPressed, start), None);
        assert_eq!(gesture.update(Input::Tick, start + HOLD_THRESHOLD), None);
        assert_eq!(
            gesture.update(Input::CtrlReleased, start + Duration::from_millis(600)),
            None
        );
    }

    #[test]
    fn plain_double_tap_does_nothing() {
        let start = Instant::now();
        let mut gesture = Gesture::default();
        assert_eq!(gesture.update(Input::CtrlPressed, start), None);
        assert_eq!(
            gesture.update(Input::CtrlReleased, start + Duration::from_millis(40)),
            None
        );
        assert_eq!(
            gesture.update(Input::CtrlPressed, start + Duration::from_millis(180)),
            None
        );
        assert_eq!(
            gesture.update(Input::CtrlReleased, start + Duration::from_millis(220)),
            None
        );
    }

    #[test]
    fn ctrl_chords_do_not_trigger_recording() {
        let start = Instant::now();
        let mut gesture = Gesture::default();
        assert_eq!(gesture.update(Input::CtrlPressed, start), None);
        assert_eq!(
            gesture.update(Input::OtherKeyPressed, start + Duration::from_millis(40)),
            None
        );
        assert_eq!(
            gesture.update(Input::Tick, start + Duration::from_millis(500)),
            None
        );
    }

    #[test]
    fn chord_after_one_tap_is_not_a_double_tap() {
        let start = Instant::now();
        let mut gesture = Gesture::default();
        assert_eq!(gesture.update(Input::CtrlPressed, start), None);
        assert_eq!(
            gesture.update(Input::CtrlReleased, start + Duration::from_millis(40)),
            None
        );
        assert_eq!(
            gesture.update(Input::CtrlPressed, start + Duration::from_millis(180)),
            None
        );
        assert_eq!(
            gesture.update(Input::OtherKeyPressed, start + Duration::from_millis(200)),
            None
        );
        assert_eq!(
            gesture.update(Input::CtrlReleased, start + Duration::from_millis(220)),
            None
        );
    }
}
