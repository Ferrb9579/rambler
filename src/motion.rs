use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    time::{Duration, Instant},
};

use incular::prelude::*;
use incular::runtime::RuntimeSpawner;
use incular::widgets::{
    HitTestBehavior,
    internal::{BlurController, OpacityController, RotationController, ScaleController},
};

use crate::controller::Phase;

pub const COPY_HOLD: Duration = Duration::from_millis(2400);
const SWAP: Duration = Duration::from_millis(85);
const SETTLE: Duration = Duration::from_millis(180);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CopyPhase {
    #[default]
    Idle,
    Leaving,
    Success,
    Returning,
}

impl CopyPhase {
    pub fn shows_check(self) -> bool {
        matches!(self, Self::Success | Self::Returning)
    }
}

#[derive(Clone, Default)]
pub struct IconMotion {
    pub scale: ScaleController,
    blur: BlurController,
    opacity: OpacityController,
}

impl IconMotion {
    pub fn widget(&self, child: Widget) -> Widget {
        FadeTransition::new(
            self.opacity.clone(),
            ScaleTransition::new(
                self.scale.clone(),
                ImageFiltered::controlled(self.blur.clone(), child),
            ),
        )
        .into()
    }

    fn leave(&self, reduced: bool) {
        if reduced {
            return;
        }
        let now = Instant::now();
        self.scale.animate_to(0.78, SWAP, now);
        self.blur.animate_to(3.0, SWAP, now);
        self.opacity.animate_to(0.15, SWAP, now);
    }

    fn enter(&self, reduced: bool) {
        if reduced {
            self.reset();
            return;
        }
        let now = Instant::now();
        self.scale.set_scale(1.18);
        self.blur.set_sigma(2.0);
        self.opacity.set_opacity(0.45);
        self.scale.animate_to(1.0, SETTLE, now);
        self.blur.animate_to(0.0, SETTLE, now);
        self.opacity.animate_to(1.0, SETTLE, now);
    }

    fn reset(&self) {
        let now = Instant::now();
        self.scale.set_scale(1.0);
        self.scale.animate_to(1.0, Duration::ZERO, now);
        self.blur.set_sigma(0.0);
        self.blur.animate_to(0.0, Duration::ZERO, now);
        self.opacity.set_opacity(1.0);
        self.opacity.animate_to(1.0, Duration::ZERO, now);
    }
}

#[derive(Clone, Default)]
struct ButtonMotion {
    scale: ScaleController,
    hovered: Rc<Cell<bool>>,
    pressed: Rc<Cell<bool>>,
}

impl ButtonMotion {
    fn reset(&self) {
        self.hovered.set(false);
        self.pressed.set(false);
        self.scale.set_scale(1.0);
        self.scale.animate_to(1.0, Duration::ZERO, Instant::now());
    }

    fn update(&self, reduced: bool) {
        let target = if reduced {
            1.0
        } else if self.pressed.get() {
            0.92
        } else if self.hovered.get() {
            1.04
        } else {
            1.0
        };
        self.scale.animate_to(
            target,
            if reduced {
                Duration::ZERO
            } else {
                Duration::from_millis(120)
            },
            Instant::now(),
        );
    }
}

#[derive(Clone)]
pub struct Motion {
    pub ready: Signal<bool>,
    spawner: Rc<RefCell<Option<RuntimeSpawner>>>,
    binding: Rc<Cell<bool>>,
    reduced: Rc<Cell<bool>>,
    buttons: Rc<RefCell<HashMap<&'static str, ButtonMotion>>>,
    pub copy_phase: Signal<CopyPhase>,
    pub copy_icon: IconMotion,
    copy_tasks: Rc<RefCell<Vec<TaskHandle>>>,
    copy_generation: Rc<Cell<u64>>,
    pub record_icon: IconMotion,
    pub record_pulse: ScaleController,
    pub processing_turn: RotationController,
    activity: Rc<Cell<Phase>>,
    activity_task: Rc<RefCell<Option<TaskHandle>>>,
    pub page_opacity: OpacityController,
    confirmations: Signal<HashMap<&'static str, u64>>,
    confirmation_tasks: Rc<RefCell<HashMap<&'static str, TaskHandle>>>,
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            ready: Signal::new(false),
            spawner: Default::default(),
            binding: Default::default(),
            reduced: Default::default(),
            buttons: Default::default(),
            copy_phase: Signal::new(CopyPhase::Idle),
            copy_icon: Default::default(),
            copy_tasks: Default::default(),
            copy_generation: Default::default(),
            record_icon: Default::default(),
            record_pulse: Default::default(),
            processing_turn: Default::default(),
            activity: Rc::new(Cell::new(Phase::Ready)),
            activity_task: Default::default(),
            page_opacity: Default::default(),
            confirmations: Signal::new(HashMap::new()),
            confirmation_tasks: Default::default(),
        }
    }
}

impl Motion {
    pub fn bind(&self, cx: &BuildContext, phase: Phase) {
        let reduced = cx.reduced_motion();
        let reduced_changed = self.reduced.replace(reduced) != reduced;
        if self.spawner.borrow().is_none() && !self.binding.replace(true) {
            let motion = self.clone();
            cx.spawn_into(async {}, move |result, runtime| {
                if result.is_ok() {
                    *motion.spawner.borrow_mut() = Some(runtime.spawner());
                    motion.ready.set(true);
                    motion.activity_cycle(false);
                }
            });
        }
        if self.activity.replace(phase) != phase || reduced_changed {
            if let Some(task) = self.activity_task.borrow_mut().take() {
                task.cancel();
            }
            self.record_pulse.set_scale(1.0);
            self.record_pulse
                .animate_to(1.0, Duration::ZERO, Instant::now());
            self.processing_turn.set_radians(0.0);
            self.processing_turn
                .animate_to(0.0, Duration::ZERO, Instant::now());
            self.record_icon.enter(self.reduced.get());
            if self.reduced.get() {
                self.copy_icon.reset();
                self.page_opacity.set_opacity(1.0);
                self.page_opacity
                    .animate_to(1.0, Duration::ZERO, Instant::now());
                for button in self.buttons.borrow().values() {
                    button.update(true);
                }
            }
            self.activity_cycle(false);
        }
    }

    fn after(&self, delay: Duration, callback: impl FnOnce() + 'static) -> Option<TaskHandle> {
        self.spawner.borrow().as_ref().map(|spawner| {
            spawner.spawn_into(
                async move {
                    tokio::time::sleep(delay).await;
                },
                move |result, _| {
                    if result.is_ok() {
                        callback();
                    }
                },
            )
        })
    }

    pub fn interactive(
        &self,
        label: &'static str,
        child: Widget,
        enabled: bool,
        tap: impl Fn() + 'static,
    ) -> Widget {
        let visual = if enabled {
            let motion = self.buttons.borrow_mut().entry(label).or_default().clone();
            let down = motion.clone();
            let up = motion.clone();
            let cancel = motion.clone();
            let enter = motion.clone();
            let exit = motion.clone();
            let reduced_down = self.reduced.clone();
            let reduced_up = self.reduced.clone();
            let reduced_cancel = self.reduced.clone();
            let reduced_enter = self.reduced.clone();
            let reduced_exit = self.reduced.clone();
            MouseRegion::new(
                GestureDetector::new(ScaleTransition::new(motion.scale, child))
                    .behavior(HitTestBehavior::Opaque)
                    .on_tap_down(move |_| {
                        down.pressed.set(true);
                        down.update(reduced_down.get());
                    })
                    .on_tap_up(move |_| {
                        up.pressed.set(false);
                        up.update(reduced_up.get());
                    })
                    .on_tap_cancel(move || {
                        cancel.pressed.set(false);
                        cancel.update(reduced_cancel.get());
                    })
                    .on_tap(tap),
            )
            .cursor(MouseCursor::Click)
            .on_enter(move |_| {
                enter.hovered.set(true);
                enter.update(reduced_enter.get());
            })
            .on_exit(move |_| {
                exit.hovered.set(false);
                exit.update(reduced_exit.get());
            })
            .into()
        } else {
            if let Some(button) = self.buttons.borrow().get(label) {
                button.reset();
            }
            child
        };
        Semantics::new(visual)
            .button(true)
            .label(label)
            .enabled(enabled)
            .into()
    }

    pub fn copied(&self) {
        self.reset_copy();
        let generation = self.copy_generation.get();
        self.copy_phase.set(CopyPhase::Leaving);
        self.copy_icon.leave(self.reduced.get());
        let next = self.clone();
        self.copy_after(
            if self.reduced.get() {
                Duration::ZERO
            } else {
                SWAP
            },
            generation,
            move || {
                next.copy_phase.set(CopyPhase::Success);
                next.copy_icon.enter(next.reduced.get());
                let returning = next.clone();
                next.copy_after(COPY_HOLD, generation, move || {
                    returning.copy_phase.set(CopyPhase::Returning);
                    returning.copy_icon.leave(returning.reduced.get());
                    let idle = returning.clone();
                    returning.copy_after(
                        if returning.reduced.get() {
                            Duration::ZERO
                        } else {
                            SWAP
                        },
                        generation,
                        move || {
                            idle.copy_phase.set(CopyPhase::Idle);
                            idle.copy_icon.enter(idle.reduced.get());
                        },
                    );
                });
            },
        );
    }

    fn copy_after(&self, delay: Duration, generation: u64, callback: impl FnOnce() + 'static) {
        let current = self.copy_generation.clone();
        if let Some(task) = self.after(delay, move || {
            if current.get() == generation {
                callback();
            }
        }) {
            self.copy_tasks.borrow_mut().push(task);
        }
    }

    pub fn reset_copy(&self) {
        self.copy_generation
            .set(self.copy_generation.get().wrapping_add(1));
        for task in self.copy_tasks.borrow_mut().drain(..) {
            task.cancel();
        }
        self.copy_phase.set(CopyPhase::Idle);
        self.copy_icon.reset();
    }

    pub fn page_enter(&self) {
        for button in self.buttons.borrow().values() {
            button.reset();
        }
        if !self.reduced.get() {
            self.page_opacity.set_opacity(0.35);
            self.page_opacity
                .animate_to(1.0, Duration::from_millis(160), Instant::now());
        }
    }

    pub fn confirm(&self, label: &'static str) {
        if let Some(task) = self.confirmation_tasks.borrow_mut().remove(label) {
            task.cancel();
        }
        let generation = self
            .confirmations
            .get_untracked()
            .get(label)
            .copied()
            .unwrap_or(0)
            + 1;
        self.confirmations.update(|items| {
            items.insert(label, generation);
        });
        let next = self.clone();
        if let Some(task) = self.after(Duration::from_secs(2), move || {
            next.confirmations.update(|items| {
                if items.get(label) == Some(&generation) {
                    items.remove(label);
                }
            });
            next.confirmation_tasks.borrow_mut().remove(label);
        }) {
            self.confirmation_tasks.borrow_mut().insert(label, task);
        }
    }

    pub fn confirmed(&self, label: &'static str) -> bool {
        self.confirmations.get().contains_key(label)
    }

    fn activity_cycle(&self, reverse: bool) {
        if self.reduced.get() || self.activity.get() == Phase::Ready {
            return;
        }
        let duration = Duration::from_millis(650);
        match self.activity.get() {
            Phase::Recording => self.record_pulse.animate_to(
                if reverse { 1.0 } else { 1.065 },
                duration,
                Instant::now(),
            ),
            Phase::Processing => {
                self.processing_turn.set_radians(0.0);
                self.processing_turn
                    .animate_to(std::f32::consts::PI, duration, Instant::now());
            }
            Phase::Ready => return,
        }
        let next = self.clone();
        *self.activity_task.borrow_mut() =
            self.after(duration, move || next.activity_cycle(!reverse));
    }
}
