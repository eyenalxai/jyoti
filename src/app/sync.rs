use std::time::{Duration, Instant};

use crate::app::App;
use crate::app::state::{StopPhase, ToastKind};
use crate::bulb::model::{BulbState, Rgb};
use crate::bulb::worker::Command;
use crate::screen::capture::{ScreenCommand, ScreenEvent};
use crate::screen::intensity::intensify;
use crate::screen::monitor::Scene;
use crate::screen::sync::{
    RestoreReport, SyncConfig, SyncFrame, SyncStatus, SyncTarget, restore_control,
};

impl App {
    pub(crate) fn start_sync(&mut self) {
        self.monitor_picker_open = false;
        if self.sync.status.is_busy() {
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

        self.sync.snapshots = self.states.clone();
        self.sync.drove_brightness.clear();
        self.sync.restore_report = None;
        self.sync.last_scene = None;
        self.sync.last_scene_at = None;
        self.sync.derived = None;
        self.sync.brightness = 0;
        self.sync.flashing = false;
        self.sync.stale = false;
        self.sync.pending_restore.clear();
        self.sync.stop = StopPhase::Idle;
        self.sync.unreachable.clear();
        self.sync.failures.clear();
        self.sync.status = SyncStatus::Starting;
        self.sync.capture_error = None;

        let sent = self.screen_handle.as_ref().is_some_and(|handle| {
            handle.send(ScreenCommand::Start {
                monitor: monitor.clone(),
                config: self.sync_config(),
            })
        });
        if !sent {
            self.restart_screen();
            if let Some(handle) = &self.screen_handle {
                let _ = handle.send(ScreenCommand::Start {
                    monitor,
                    config: self.sync_config(),
                });
            }
        }
    }

    pub(crate) fn sync_config(&self) -> SyncConfig {
        SyncConfig {
            gain: self.sync.gain,
            intensity: self.sync.colour_intensity,
            targets: self
                .bulbs
                .iter()
                .filter(|bulb| !self.sync.unreachable.contains(&bulb.id))
                .filter_map(|bulb| {
                    Some(SyncTarget {
                        id: bulb.id.clone(),
                        sender: self.workers.get(&bulb.id)?.clone(),
                        on: self.states.get(&bulb.id).and_then(|state| state.power) == Some(true),
                    })
                })
                .collect(),
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
        if !self.sync.status.is_running() {
            return;
        }
        self.sync.status = SyncStatus::Paused;
        if let Some(handle) = &self.screen_handle {
            let _ = handle.send(ScreenCommand::Pause);
        }
    }

    pub(crate) fn resume_sync(&mut self) {
        if self.sync.status != SyncStatus::Paused {
            return;
        }
        self.sync.status = SyncStatus::Running;
        self.sync.stale = false;
        self.sync.last_scene_at = Some(Instant::now());
        if let Some(handle) = &self.screen_handle {
            let _ = handle.send(ScreenCommand::Resume);
        }
    }

    pub(crate) fn stop_sync(&mut self) {
        if !self.sync.status.is_active() {
            return;
        }
        self.begin_restore();
    }

    pub(crate) fn begin_restore(&mut self) {
        self.sync.status = SyncStatus::Stopping;
        self.sync.pending_restore.clear();
        self.sync.stop = StopPhase::Waiting {
            until: Instant::now() + Duration::from_millis(2_000),
        };

        let sent = self
            .screen_handle
            .as_ref()
            .is_some_and(|handle| handle.send(ScreenCommand::Stop));
        if !sent {
            self.start_restore();
        }
    }

    pub(crate) fn start_restore(&mut self) {
        if !matches!(self.sync.stop, StopPhase::Waiting { .. }) {
            return;
        }
        let restore_deadline = Instant::now() + Duration::from_millis(2_000);
        self.sync.stop = StopPhase::Restoring {
            deadline: restore_deadline,
        };

        let mut report = RestoreReport {
            restored: 0,
            not_restored: Vec::new(),
            total: self.bulbs.len(),
        };
        for bulb in &self.bulbs {
            if !self.sync.snapshots.contains_key(&bulb.id) {
                report.not_restored.push(bulb.name.clone());
            }
        }

        let snapshots: Vec<(String, BulbState)> = self
            .sync
            .snapshots
            .iter()
            .map(|(id, state)| (id.clone(), state.clone()))
            .collect();
        for (id, state) in snapshots {
            let (color, temperature) = match restore_control(&state) {
                Some(crate::screen::sync::RestoreControl::Color(color)) => (Some(color), None),
                Some(crate::screen::sync::RestoreControl::Temperature(temperature)) => {
                    (None, Some(temperature))
                }
                None => (None, None),
            };
            let brightness = if self.sync.drove_brightness.contains(&id) {
                state.brightness
            } else {
                None
            };

            if color.is_none() && temperature.is_none() && brightness.is_none() {
                if let Some(bulb) = self.bulbs.iter().find(|bulb| bulb.id == id) {
                    report.not_restored.push(bulb.name.clone());
                }
                continue;
            }
            if let Some(sender) = self.workers.get(&id) {
                let _ = sender.send(Command::Restore {
                    color,
                    temperature,
                    brightness,
                    deadline: restore_deadline,
                });
                self.sync.pending_restore.insert(id);
            } else if let Some(bulb) = self.bulbs.iter().find(|bulb| bulb.id == id) {
                report.not_restored.push(bulb.name.clone());
            }
        }

        self.sync.restore_report = Some(report);
        if self.sync.pending_restore.is_empty() {
            self.finish_restore();
        }
    }

    pub(crate) fn finish_restore(&mut self) {
        if self.sync.stop == StopPhase::Idle {
            return;
        }
        self.sync.stop = StopPhase::Idle;
        if let Some(report) = self.sync.restore_report.as_mut() {
            for id in &self.sync.pending_restore {
                if let Some(bulb) = self.bulbs.iter().find(|bulb| &bulb.id == id)
                    && !report.not_restored.contains(&bulb.name)
                {
                    report.not_restored.push(bulb.name.clone());
                }
            }
        }
        self.sync.pending_restore.clear();
        self.sync.status = SyncStatus::Idle;
        let report = self.sync.restore_report.take();
        let message = report
            .as_ref()
            .map_or_else(|| "Screen sync stopped.".to_owned(), RestoreReport::message);
        let kind = if report
            .as_ref()
            .is_some_and(|report| report.not_restored.is_empty())
        {
            ToastKind::Success
        } else {
            ToastKind::Error
        };
        self.notify(message, kind);
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
                if self.sync.status == SyncStatus::Starting {
                    self.sync.status = SyncStatus::Running;
                }
                self.note_frame(scene, intensify(scene.color, self.sync.colour_intensity));
            }
            ScreenEvent::Started => {
                self.sync.capture_error = None;
                self.sync.status = SyncStatus::Running;
            }
            ScreenEvent::Stopped { drove } => match self.sync.stop {
                StopPhase::Waiting { .. } => {
                    self.sync.drove_brightness.extend(drove);
                    self.start_restore();
                }
                StopPhase::Idle => self.sync.status = SyncStatus::Idle,
                StopPhase::Restoring { .. } => {}
            },
            ScreenEvent::Error(message) => {
                if self.sync.status.is_running() {
                    if !self.sync.stale {
                        self.notify(message.clone(), ToastKind::Error);
                    }
                    self.sync.stale = true;
                } else {
                    self.sync.capture_error = Some(message.clone());
                    self.notify(message, ToastKind::Error);
                }
            }
            ScreenEvent::Fatal(message) => {
                if self.sync.status.is_running() || self.sync.status == SyncStatus::Stopping {
                    self.sync.status = SyncStatus::Error(message.clone());
                    self.sync.stale = true;
                } else {
                    self.sync.capture_error = Some(message.clone());
                }
                self.notify(message, ToastKind::Error);
            }
        }
    }

    pub(crate) fn note_sync_frame(&mut self, frame: SyncFrame) {
        self.sync.capture_error = None;
        if self.sync.status == SyncStatus::Starting {
            self.sync.status = SyncStatus::Running;
        }
        self.note_frame(frame.scene, frame.output);
        self.sync.brightness = frame.brightness;
        self.sync.flashing = frame.flashing;
        self.sync.drove_brightness.extend(frame.drove_brightness);
    }

    fn note_frame(&mut self, scene: Scene, output: Rgb) {
        self.sync.last_scene = Some(scene);
        self.sync.derived = Some(output);
        self.sync.last_scene_at = Some(Instant::now());
        self.sync.stale = false;
    }
}
