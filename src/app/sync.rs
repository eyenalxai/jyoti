pub(crate) mod restore;
pub(crate) mod run;

use std::time::Instant;

use crate::app::App;
use crate::app::state::ToastKind;
use crate::app::sync::restore::{RestorePlan, RestoreRequest};
use crate::app::sync::run::{RunAdvance, RunEvent, RunPhase, StopStage};
use crate::app::view::Overlay;
use crate::bulb::model::Rgb;
use crate::bulb::runtime::Runtimes;
use crate::bulb::worker::Command;
use crate::screen::capture::{ScreenCommand, ScreenEvent};
use crate::screen::intensity::intensify;
use crate::screen::monitor::Scene;
use crate::screen::sync::{SyncConfig, SyncFrame, SyncTarget};

impl App {
    pub(crate) fn start_sync(&mut self) {
        self.close_overlay(Overlay::MonitorPicker);
        if !self.sync.controls().start {
            return;
        }
        if self.runtimes.is_empty() {
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
        self.runtimes.begin_run();
        self.flush_pending();
        self.sync.restore.begin(&self.runtimes);

        self.sync.last_scene = None;
        self.sync.last_scene_at = None;
        self.sync.derived = None;
        self.sync.brightness = 0;
        self.sync.flash_until = None;
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
            self.runtimes.end_run();
            self.notify(message, ToastKind::Error);
        }
    }

    pub(crate) fn sync_config(&self) -> SyncConfig {
        SyncConfig {
            gain: self.sync.gain,
            intensity: self.sync.colour_intensity,
            targets: sync_targets(&self.runtimes, &self.sync.restore),
        }
    }

    pub(crate) fn configure_sync(&self) {
        if !self.sync.status.is_in_run() {
            return;
        }
        if let Some(handle) = &self.screen_handle {
            let _ = handle.send(ScreenCommand::Configure(self.sync_config()));
        }
    }

    pub(crate) fn pause_sync(&mut self) {
        if let Some(handle) = &self.screen_handle {
            let _ = handle.send(ScreenCommand::Pause);
        }
    }

    pub(crate) fn resume_sync(&mut self) {
        if let Some(handle) = &self.screen_handle {
            let _ = handle.send(ScreenCommand::Resume);
        }
    }

    pub(crate) fn stop_sync(&mut self) {
        let now = Instant::now();
        if self.sync.apply(RunEvent::Stop).is_err() {
            return;
        }
        self.request_capture_stop();
        self.advance_sync(now);
    }

    fn request_capture_stop(&mut self) {
        let Some(generation) = self.sync.status.generation() else {
            return;
        };
        let sent = self.screen_handle.as_ref().is_some_and(|handle| {
            handle.halt(generation);
            handle.send(ScreenCommand::Stop)
        });
        if !sent {
            let _ = self.sync.apply(RunEvent::Stopped(generation));
        }
    }

    pub(crate) fn advance_sync(&mut self, now: Instant) -> bool {
        match self.sync.status.advance(now) {
            RunAdvance::Unchanged => false,
            RunAdvance::BeginRestore { deadline } => {
                self.dispatch_or_finish(deadline);
                true
            }
            RunAdvance::RestoreDue => {
                self.finish_restore();
                true
            }
        }
    }

    fn dispatch_or_finish(&mut self, deadline: Instant) {
        self.dispatch_restore(deadline);
        if !self.sync.restore.is_open() {
            self.finish_restore();
        }
    }

    fn dispatch_restore(&mut self, deadline: Instant) {
        let requests = self.sync.restore.requests();
        for request in requests {
            match request {
                RestoreRequest::Restore { id, controls } => {
                    let Some(sender) = self.runtimes.worker(&id) else {
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
    }

    pub(crate) fn finish_restore(&mut self) {
        let Some(generation) = self.sync.status.generation() else {
            return;
        };
        if self.sync.apply(RunEvent::Restored(generation)).is_err() {
            return;
        }
        self.runtimes.end_run();
        let report = self.sync.restore.finish();
        let kind = if report.not_restored.is_empty() {
            ToastKind::Success
        } else {
            ToastKind::Error
        };
        self.notify(report.message(), kind);
    }

    pub(crate) fn select_monitor(&mut self, name: String) {
        self.close_overlay(Overlay::MonitorPicker);
        self.sync.selected_monitor = Some(name.clone());
        if self.sync.status.is_in_run()
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
            ScreenEvent::Phase(phase) => {
                if self.sync.report(phase) && phase.is_running() {
                    self.sync.capture_error = None;
                    self.sync.last_scene_at = Some(Instant::now());
                }
            }
            ScreenEvent::Stopped { generation, drove } => {
                if self.sync.apply(RunEvent::Stopped(generation)).is_ok() {
                    self.sync.restore.record(&drove);
                    match self.sync.status.stop_stage() {
                        Some(StopStage::AwaitingCapture { .. }) => {
                            self.advance_sync(Instant::now());
                        }
                        Some(StopStage::Restoring { deadline }) => {
                            self.dispatch_or_finish(deadline);
                        }
                        None => {}
                    }
                }
            }
            ScreenEvent::Error(message) => {
                if self.sync.status.is_in_run() {
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
                if failed {
                    self.runtimes.end_run();
                } else {
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
        if !matches!(self.sync.status, RunPhase::Running(_)) {
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

fn sync_targets(runtimes: &Runtimes, restore: &RestorePlan) -> Vec<SyncTarget> {
    runtimes
        .iter()
        .filter(|runtime| !runtime.is_run_unreachable())
        .filter(|runtime| restore.covers(runtime.id()))
        .filter_map(|runtime| {
            Some(SyncTarget {
                id: runtime.id().to_owned(),
                sender: runtime.worker()?.capture_sender(),
                on: runtime.state().power == Some(true),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;
    use std::sync::mpsc::channel;

    use super::sync_targets;
    use crate::app::sync::restore::RestorePlan;
    use crate::bulb::health::HealthEvent;
    use crate::bulb::model::{Bulb, BulbState, ColorMode, Rgb};
    use crate::bulb::runtime::Runtimes;
    use crate::bulb::worker::test_handle;

    fn bulb(id: &str, name: &str) -> Bulb {
        Bulb {
            id: id.to_owned(),
            name: name.to_owned(),
            address: IpAddr::from([192, 168, 1, 42]),
            model: None,
        }
    }

    fn runtimes(entries: &[(&str, &str)]) -> Runtimes {
        Runtimes::from_bulbs(entries.iter().map(|(id, name)| bulb(id, name)).collect())
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

    fn add_worker(runtimes: &mut Runtimes, id: &str) {
        let (sender, _commands) = channel();
        assert!(runtimes.set_worker(id, test_handle(sender)));
    }

    #[test]
    fn a_bulb_without_a_plan_entry_is_never_a_target() {
        let mut runtimes = runtimes(&[("desk", "Desk lamp"), ("shelf", "Shelf lamp")]);
        runtimes.set_state("desk", state(true));
        runtimes.set_state("shelf", state(true));
        add_worker(&mut runtimes, "desk");
        add_worker(&mut runtimes, "shelf");
        let mut restore = RestorePlan::default();
        restore.observe("desk", "Desk lamp", &state(true));

        let targets = sync_targets(&runtimes, &restore);

        let ids: Vec<String> = targets.into_iter().map(|target| target.id).collect();
        assert_eq!(ids, vec!["desk".to_owned()]);
    }

    #[test]
    fn a_bulb_the_run_marked_is_not_a_target() {
        let mut runtimes = runtimes(&[("desk", "Desk lamp"), ("shelf", "Shelf lamp")]);
        runtimes.set_state("desk", state(true));
        runtimes.set_state("shelf", state(true));
        add_worker(&mut runtimes, "desk");
        add_worker(&mut runtimes, "shelf");
        let mut restore = RestorePlan::default();
        restore.observe("desk", "Desk lamp", &state(true));
        restore.observe("shelf", "Shelf lamp", &state(true));
        for message in ["one", "two", "three"] {
            runtimes.apply_health("shelf", HealthEvent::Error(message.to_owned()), true);
        }

        let targets = sync_targets(&runtimes, &restore);

        let ids: Vec<String> = targets.into_iter().map(|target| target.id).collect();
        assert_eq!(ids, vec!["desk".to_owned()]);
    }

    #[test]
    fn a_standing_failure_is_retried_by_the_run() {
        let mut runtimes = runtimes(&[("desk", "Desk lamp")]);
        runtimes.set_state("desk", state(true));
        add_worker(&mut runtimes, "desk");
        let mut restore = RestorePlan::default();
        restore.observe("desk", "Desk lamp", &state(true));
        runtimes.apply_health("desk", HealthEvent::Error("no answer".to_owned()), false);

        let targets = sync_targets(&runtimes, &restore);

        let ids: Vec<String> = targets.into_iter().map(|target| target.id).collect();
        assert_eq!(ids, vec!["desk".to_owned()]);
    }

    #[test]
    fn a_bulb_without_a_worker_is_not_a_target() {
        let mut runtimes = runtimes(&[("desk", "Desk lamp")]);
        runtimes.set_state("desk", state(true));
        let mut restore = RestorePlan::default();
        restore.observe("desk", "Desk lamp", &state(true));

        let targets = sync_targets(&runtimes, &restore);

        assert!(targets.is_empty());
    }
}
