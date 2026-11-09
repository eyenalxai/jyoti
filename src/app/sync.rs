pub(crate) mod restore;
pub(crate) mod run;

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Sender;
use std::time::Instant;

use crate::app::App;
use crate::app::state::ToastKind;
use crate::app::sync::restore::{RESTORE_WINDOW, RestorePlan, RestoreRequest, StopPhase};
use crate::app::sync::run::RunEvent;
use crate::bulb::model::{Bulb, BulbState, Rgb};
use crate::bulb::worker::Command;
use crate::screen::capture::{ScreenCommand, ScreenEvent};
use crate::screen::intensity::intensify;
use crate::screen::monitor::Scene;
use crate::screen::sync::{SyncConfig, SyncFrame, SyncTarget};

impl App {
    pub(crate) fn start_sync(&mut self) {
        self.monitor_picker_open = false;
        if !self.sync.controls().start {
            return;
        }
        if self.bulbs.is_empty() {
            self.notify("Add a bulb before starting screen sync.", ToastKind::Error);
            return;
        }
        let monitor = self.sync.selected_monitor.clone().or_else(|| {
            self.sync
                .monitors
                .first()
                .map(|monitor| monitor.name.clone())
        });
        let Some(monitor) = monitor else {
            self.notify("No monitor is available for screen sync.", ToastKind::Error);
            return;
        };

        let generation = self.sync.next_generation;
        self.sync.next_generation = generation.wrapping_add(1);
        if self.sync.apply(RunEvent::Start(generation)).is_err() {
            return;
        }
        self.flush_pending();
        self.sync.restore.begin(&self.bulbs, &self.states);

        self.sync.last_scene = None;
        self.sync.last_scene_at = None;
        self.sync.derived = None;
        self.sync.brightness = 0;
        self.sync.flash_until = None;
        self.sync.stop = StopPhase::Idle;
        self.sync.stop_seen = false;
        self.sync.unreachable.clear();
        self.sync.failures.clear();
        self.sync.capture_error = None;
        self.sync.capture_failed = false;

        let config = self.sync_config();
        let mut sent = self.screen_handle.as_ref().is_some_and(|handle| {
            handle.send(ScreenCommand::Start {
                generation,
                monitor: monitor.clone(),
                config: config.clone(),
            })
        });
        if !sent {
            self.restart_screen();
            sent = self.screen_handle.as_ref().is_some_and(|handle| {
                handle.send(ScreenCommand::Start {
                    generation,
                    monitor,
                    config,
                })
            });
        }
        if !sent {
            let message = "Screen capture did not start.".to_owned();
            let _ = self.sync.apply(RunEvent::Fail {
                generation,
                message: message.clone(),
            });
            self.notify(message, ToastKind::Error);
        }
    }

    pub(crate) fn sync_config(&self) -> SyncConfig {
        SyncConfig {
            gain: self.sync.gain,
            intensity: self.sync.colour_intensity,
            targets: sync_targets(
                &self.bulbs,
                &self.states,
                &self.workers,
                &self.sync.unreachable,
                &self.sync.restore,
            ),
        }
    }

    pub(crate) fn configure_sync(&self) {
        if !self.sync.status.is_running() {
            return;
        }
        if let Some(handle) = &self.screen_handle {
            let _ = handle.send(ScreenCommand::Configure(self.sync_config()));
        }
    }

    pub(crate) fn pause_sync(&mut self) {
        if self.sync.apply(RunEvent::Pause).is_err() {
            return;
        }
        if let Some(handle) = &self.screen_handle {
            let _ = handle.send(ScreenCommand::Pause);
        }
    }

    pub(crate) fn resume_sync(&mut self) {
        if self.sync.apply(RunEvent::Resume).is_err() {
            return;
        }
        self.sync.last_scene_at = Some(Instant::now());
        if let Some(handle) = &self.screen_handle {
            let _ = handle.send(ScreenCommand::Resume);
        }
    }

    pub(crate) fn stop_sync(&mut self) {
        if self.sync.apply(RunEvent::Stop).is_err() {
            return;
        }
        self.begin_restore();
    }

    pub(crate) fn begin_restore(&mut self) {
        self.sync.stop = StopPhase::Waiting {
            until: Instant::now() + RESTORE_WINDOW,
        };
        self.sync.stop_seen = false;

        let generation = self.sync.status.generation().unwrap_or(0);
        let sent = self.screen_handle.as_ref().is_some_and(|handle| {
            handle.halt(generation);
            handle.send(ScreenCommand::Stop)
        });
        if !sent {
            self.sync.stop_seen = true;
            self.start_restore();
        }
    }

    pub(crate) fn start_restore(&mut self) {
        if !matches!(self.sync.stop, StopPhase::Waiting { .. }) {
            return;
        }
        let deadline = Instant::now() + RESTORE_WINDOW;
        self.sync.stop = StopPhase::Restoring { deadline };
        self.dispatch_restore(deadline);
    }

    fn dispatch_restore(&mut self, deadline: Instant) {
        let requests = self.sync.restore.requests();
        for request in requests {
            match request {
                RestoreRequest::Restore { id, controls } => {
                    let Some(sender) = self.workers.get(&id) else {
                        self.sync.restore.mark_failed(&id);
                        continue;
                    };
                    let (color, temperature) = controls.color_and_temperature();
                    let sent = sender
                        .send(Command::Restore {
                            generation: self.sync.status.generation().unwrap_or(0),
                            color,
                            temperature,
                            brightness: controls.brightness,
                            deadline,
                        })
                        .is_ok();
                    if sent {
                        self.sync.restore.mark_waiting(&id);
                    } else {
                        self.sync.restore.mark_failed(&id);
                    }
                }
                RestoreRequest::MissingValue { id } => self.sync.restore.mark_failed(&id),
            }
        }
        if self.sync.stop_seen
            && !self.sync.restore.is_open()
            && matches!(self.sync.stop, StopPhase::Restoring { .. })
        {
            self.finish_restore();
        }
    }

    pub(crate) fn finish_restore(&mut self) {
        if self.sync.stop == StopPhase::Idle {
            return;
        }
        self.sync.stop = StopPhase::Idle;
        let _ = self.sync.apply(RunEvent::Restored);
        let report = self.sync.restore.finish();
        let kind = if report.not_restored.is_empty() {
            ToastKind::Success
        } else {
            ToastKind::Error
        };
        self.notify(report.message(), kind);
    }

    pub(crate) fn select_monitor(&mut self, name: String) {
        self.monitor_picker_open = false;
        self.sync.selected_monitor = Some(name.clone());
        if self.sync.status.is_running()
            && let Some(handle) = &self.screen_handle
        {
            let _ = handle.send(ScreenCommand::SelectMonitor(name));
        }
    }

    pub(crate) fn restart_screen(&mut self) {
        self.sync.capture_error = None;
        let Some(waker) = self.waker.clone() else {
            return;
        };
        let handle = crate::screen::capture::ScreenHandle::start(waker);
        let _ = handle.send(ScreenCommand::ListMonitors);
        self.screen_handle = Some(handle);
    }

    pub(crate) fn handle_screen_event(&mut self, event: ScreenEvent) {
        match event {
            ScreenEvent::Monitors(monitors) => {
                self.sync.capture_error = None;
                self.sync.monitors = monitors;
                let valid =
                    self.sync.selected_monitor.as_ref().is_some_and(|selected| {
                        self.sync.monitors.iter().any(|m| &m.name == selected)
                    });
                if !valid {
                    self.sync.selected_monitor = self.sync.monitors.first().map(|m| m.name.clone());
                }
            }
            ScreenEvent::Scene(scene) => {
                self.sync.capture_error = None;
                if self.capture_once {
                    eprintln!(
                        "capture: {} brightness {:.3}",
                        scene.color.hex(),
                        scene.brightness
                    );
                }
                self.note_frame(scene, intensify(scene.color, self.sync.colour_intensity));
            }
            ScreenEvent::Started { generation } => {
                self.sync.capture_error = None;
                let _ = self.sync.apply(RunEvent::Started(generation));
            }
            ScreenEvent::Stopped { generation, drove } => {
                if self.sync.apply(RunEvent::Stopped(generation)).is_ok() {
                    self.sync.stop_seen = true;
                    self.sync.restore.record(&drove);
                    match self.sync.stop {
                        StopPhase::Waiting { .. } => self.start_restore(),
                        StopPhase::Restoring { deadline } => self.dispatch_restore(deadline),
                        StopPhase::Idle => {}
                    }
                }
            }
            ScreenEvent::Error(message) => {
                if self.sync.status.is_running() {
                    if !self.sync.capture_failed {
                        self.notify(message, ToastKind::Error);
                        self.sync.capture_failed = true;
                    }
                } else {
                    self.sync.capture_error = Some(message.clone());
                    self.notify(message, ToastKind::Error);
                }
            }
            ScreenEvent::Fatal(message) => {
                let mut failed = false;
                if let Some(generation) = self.sync.status.generation()
                    && self
                        .sync
                        .apply(RunEvent::Fail {
                            generation,
                            message: message.clone(),
                        })
                        .is_ok()
                {
                    failed = true;
                }
                if !failed {
                    self.sync.capture_error = Some(message.clone());
                }
                self.notify(message, ToastKind::Error);
            }
        }
    }

    pub(crate) fn note_sync_frame(&mut self, frame: SyncFrame) {
        let SyncFrame {
            scene,
            output,
            brightness,
            flash_until,
            drove,
        } = frame;
        if self.sync.apply(RunEvent::Frame).is_err() {
            return;
        }
        self.sync.restore.record(&drove);
        self.sync.capture_error = None;
        self.note_frame(scene, output);
        self.sync.brightness = brightness;
        self.sync.flash_until = flash_until;
    }

    fn note_frame(&mut self, scene: Scene, output: Rgb) {
        self.sync.last_scene = Some(scene);
        self.sync.derived = Some(output);
        self.sync.last_scene_at = Some(Instant::now());
        self.sync.capture_failed = false;
    }
}

fn sync_targets(
    bulbs: &[Bulb],
    states: &HashMap<String, BulbState>,
    workers: &HashMap<String, Sender<Command>>,
    unreachable: &HashSet<String>,
    restore: &RestorePlan,
) -> Vec<SyncTarget> {
    bulbs
        .iter()
        .filter(|bulb| !unreachable.contains(&bulb.id))
        .filter(|bulb| restore.covers(&bulb.id))
        .filter_map(|bulb| {
            Some(SyncTarget {
                id: bulb.id.clone(),
                sender: workers.get(&bulb.id)?.clone(),
                on: states.get(&bulb.id).and_then(|state| state.power) == Some(true),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::sync::mpsc::channel;

    use super::sync_targets;
    use crate::app::sync::restore::RestorePlan;
    use crate::bulb::model::{Bulb, BulbState, ColorMode, Rgb};
    use crate::bulb::worker::Command;

    fn bulb(id: &str, name: &str) -> Bulb {
        Bulb {
            id: id.to_owned(),
            name: name.to_owned(),
            address: "192.168.1.42".to_owned(),
            model: None,
        }
    }

    fn state(on: bool) -> BulbState {
        BulbState {
            power: Some(on),
            brightness: Some(40),
            color: Some(Rgb::new(255, 0, 0)),
            mode: ColorMode::Color,
            ..BulbState::default()
        }
    }

    #[test]
    fn a_bulb_without_a_plan_entry_is_never_a_target() {
        let desk = bulb("desk", "Desk lamp");
        let shelf = bulb("shelf", "Shelf lamp");
        let bulbs = vec![desk, shelf];
        let states = HashMap::from([
            ("desk".to_owned(), state(true)),
            ("shelf".to_owned(), state(true)),
        ]);
        let (desk_sender, _desk_commands) = channel();
        let (shelf_sender, _shelf_commands) = channel();
        let workers = HashMap::from([
            ("desk".to_owned(), desk_sender),
            ("shelf".to_owned(), shelf_sender),
        ]);
        let mut restore = RestorePlan::default();
        restore.observe("desk", "Desk lamp", &state(true));

        let targets = sync_targets(&bulbs, &states, &workers, &HashSet::new(), &restore);

        let ids: Vec<String> = targets.into_iter().map(|target| target.id).collect();
        assert_eq!(ids, vec!["desk".to_owned()]);
    }

    #[test]
    fn an_unreachable_bulb_is_not_a_target() {
        let desk = bulb("desk", "Desk lamp");
        let shelf = bulb("shelf", "Shelf lamp");
        let bulbs = vec![desk, shelf];
        let states = HashMap::from([
            ("desk".to_owned(), state(true)),
            ("shelf".to_owned(), state(true)),
        ]);
        let (desk_sender, _desk_commands) = channel();
        let (shelf_sender, _shelf_commands) = channel();
        let workers = HashMap::from([
            ("desk".to_owned(), desk_sender),
            ("shelf".to_owned(), shelf_sender),
        ]);
        let mut restore = RestorePlan::default();
        restore.observe("desk", "Desk lamp", &state(true));
        restore.observe("shelf", "Shelf lamp", &state(true));
        let unreachable = HashSet::from(["shelf".to_owned()]);

        let targets = sync_targets(&bulbs, &states, &workers, &unreachable, &restore);

        let ids: Vec<String> = targets.into_iter().map(|target| target.id).collect();
        assert_eq!(ids, vec!["desk".to_owned()]);
    }

    #[test]
    fn a_bulb_without_a_worker_is_not_a_target() {
        let desk = bulb("desk", "Desk lamp");
        let bulbs = vec![desk];
        let states = HashMap::from([("desk".to_owned(), state(true))]);
        let workers = HashMap::<String, std::sync::mpsc::Sender<Command>>::new();
        let mut restore = RestorePlan::default();
        restore.observe("desk", "Desk lamp", &state(true));

        let targets = sync_targets(&bulbs, &states, &workers, &HashSet::new(), &restore);

        assert!(targets.is_empty());
    }
}
