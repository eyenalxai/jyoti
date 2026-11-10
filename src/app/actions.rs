use std::time::{Duration, Instant};

use rune::{TextEdit, Tx};

use crate::app::App;
use crate::app::state::{Dashboard, FormMode, FormState, Screen, Toast, ToastKind};
use crate::app::sync::run::StopStage;
use crate::bulb::aggregate::AggregatePower;
use crate::bulb::health::HealthEvent;
use crate::bulb::model::{
    Bulb, BulbField, BulbListIssue, BulbState, ColorMode, NEUTRAL_TEMPERATURE, Rgb, new_id,
    trim_model, validate_draft,
};
use crate::bulb::registry::RegistryError;
use crate::bulb::selection::{Selection, resolve_selection, selection_after_add};
use crate::bulb::worker::{Command, WorkerEvent, WorkerExit};
use crate::screen::capture::ScreenCommand;
use crate::screen::gain::normalize_sync_gain;
use crate::screen::intensity::{intensify, normalize_sync_intensity};

impl App {
    fn ensure_sync_stopped(&mut self, action: &str) -> bool {
        if !self.sync.controls().manual {
            self.notify(
                format!("Stop screen sync before you {action}."),
                ToastKind::Info,
            );
            false
        } else {
            true
        }
    }

    pub(crate) fn spawn_worker(&mut self, bulb: &Bulb) {
        if let Some(previous) = self.workers.remove(&bulb.id) {
            previous.stop();
        }
        let Some(waker) = self.waker.clone() else {
            return;
        };
        let handle = crate::bulb::worker::spawn(
            bulb.clone(),
            self.registry.token_store().clone(),
            self.config.clone(),
            self.worker_sender.clone(),
            waker,
        );
        self.health.remove(&bulb.id);
        self.workers.insert(bulb.id.clone(), handle);
    }

    pub(crate) fn handle_worker_event(&mut self, event: WorkerEvent) -> bool {
        let id = event.id();
        let worker = event.worker();
        if !self.bulbs.iter().any(|bulb| bulb.id == id) {
            return false;
        }
        if !self
            .workers
            .get(id)
            .is_some_and(|handle| handle.is_current(worker))
        {
            return false;
        }
        let mut reconfigure = false;
        let in_run = self.sync.status.is_running();
        let changed = match event {
            WorkerEvent::State {
                id,
                state,
                observed_at,
                ..
            } => {
                let health_changed = self.health.apply(&id, HealthEvent::State, in_run);
                reconfigure |= health_changed;
                if self.edits.accepts(&id, observed_at) {
                    let (was_on, is_on, merged) = {
                        let current = self.states.entry(id.clone()).or_default();
                        let was_on = current.power == Some(true);
                        current.apply(&state);
                        (was_on, current.power == Some(true), current.clone())
                    };
                    if self.sync.status.is_running()
                        && let Some(bulb) = self.bulbs.iter().find(|bulb| bulb.id == id)
                    {
                        reconfigure |= self.sync.restore.observe(&id, &bulb.name, &merged);
                    }
                    reconfigure |= was_on != is_on;
                    true
                } else {
                    health_changed
                }
            }
            WorkerEvent::Sent { id, .. } => {
                let changed = self.health.apply(&id, HealthEvent::Sent, in_run);
                reconfigure = changed;
                changed
            }
            WorkerEvent::Error { id, error, .. } => {
                let message = error.to_string();
                let changed = self
                    .health
                    .apply(&id, HealthEvent::Error(message.clone()), in_run);
                self.notify(message, ToastKind::Error);
                reconfigure = changed;
                true
            }
            WorkerEvent::Restored {
                id, generation, ok, ..
            } => {
                let current = self.sync.status.generation() == Some(generation)
                    && matches!(
                        self.sync.status.stop_stage(),
                        Some(StopStage::Restoring { .. })
                    );
                if current {
                    if ok {
                        reconfigure |= self.health.apply(&id, HealthEvent::Sent, in_run);
                    }
                    self.sync.restore.acknowledge(&id, ok);
                    if !self.sync.restore.is_open() {
                        self.finish_restore();
                    }
                }
                current
            }
            WorkerEvent::Reachable { id, reachable, .. } => {
                let changed = self
                    .health
                    .apply(&id, HealthEvent::Reachable(reachable), in_run);
                reconfigure = changed;
                changed
            }
            WorkerEvent::Exited { id, exit, .. } => {
                self.workers.remove(&id);
                reconfigure = true;
                if exit == WorkerExit::Failed {
                    self.health.apply(&id, HealthEvent::Exited, in_run)
                } else {
                    false
                }
            }
        };
        if reconfigure {
            self.configure_sync();
        }
        changed
    }

    pub(crate) fn refresh_all(&mut self, tx: &mut Tx) {
        if !self.ensure_sync_stopped("refresh the bulb list") {
            return;
        }
        match self.registry.load() {
            Ok(bulbs) => {
                self.startup_error = None;
                let ids: Vec<String> = self.workers.keys().cloned().collect();
                for id in ids {
                    if let Some(handle) = self.workers.remove(&id) {
                        handle.stop();
                    }
                }
                self.bulbs = bulbs;
                let keep: std::collections::HashSet<String> =
                    self.bulbs.iter().map(|bulb| bulb.id.clone()).collect();
                self.states.retain(|id, _| keep.contains(id));
                self.health.retain(&keep);
                self.edits.clear();
                self.selection = resolve_selection(&self.bulbs, self.selection.as_ref());
                let bulbs = self.bulbs.clone();
                for bulb in bulbs {
                    self.spawn_worker(&bulb);
                }
                self.configure_sync();
            }
            Err(error) => {
                self.startup_error = Some(error.to_string());
                self.notify(error.to_string(), ToastKind::Error);
            }
        }

        if self.bulbs.is_empty() {
            self.notify(
                "No bulbs yet. Add one from the control screen.",
                ToastKind::Info,
            );
            tx.wake_after(Duration::from_secs(4));
            return;
        }
        tx.wake_after(Duration::ZERO);
    }

    pub(crate) fn toggle_power(&mut self) {
        if self.bulbs.is_empty() {
            return;
        }
        let target = match &self.selection {
            Some(Selection::All) => !matches!(self.aggregate().power, AggregatePower::On),
            Some(Selection::Bulb(id)) => self
                .states
                .get(id)
                .and_then(|state| state.power)
                .is_none_or(|power| !power),
            None => return,
        };
        let ids = self.selected_ids();
        for id in &ids {
            self.set_local_power(id, target);
            self.send(id, Command::SetPower(target));
        }
    }

    pub(crate) fn set_mode_white(&mut self) {
        if !self.sync.controls().manual {
            self.notify(
                "Screen sync owns colour and temperature while it runs.",
                ToastKind::Info,
            );
            return;
        }
        let ids = self.selected_ids();
        for id in ids {
            let temperature = self
                .states
                .get(&id)
                .and_then(|state| state.temperature)
                .unwrap_or(NEUTRAL_TEMPERATURE);
            if let Some(state) = self.states.get_mut(&id) {
                state.mode = ColorMode::Temperature;
                state.temperature = Some(temperature);
            }
            self.edits.touch(&id);
            self.send(&id, Command::SetTemperature(temperature));
        }
    }

    pub(crate) fn set_mode_color(&mut self) {
        if !self.sync.controls().manual {
            self.notify(
                "Screen sync owns colour and temperature while it runs.",
                ToastKind::Info,
            );
            return;
        }
        let ids = self.selected_ids();
        for id in ids {
            let color = self
                .states
                .get(&id)
                .and_then(|state| state.color)
                .unwrap_or(Rgb::WHITE);
            if let Some(state) = self.states.get_mut(&id) {
                state.mode = ColorMode::Color;
                state.color = Some(color);
            }
            self.edits.touch(&id);
            self.send(&id, Command::SetColor(color));
        }
    }

    pub(crate) fn set_brightness(&mut self, value: u8) {
        if !self.sync.controls().manual {
            return;
        }
        let ids = self.selected_ids();
        for id in ids {
            if let Some(state) = self.states.get_mut(&id) {
                state.brightness = Some(value);
            } else {
                self.states.insert(
                    id.clone(),
                    BulbState {
                        brightness: Some(value),
                        ..Default::default()
                    },
                );
            }
            self.edits.touch(&id);
            self.dashboard.pending_brightness.insert(id, value);
        }
        self.mark_pending();
    }

    pub(crate) fn set_temperature(&mut self, value: u16) {
        if !self.sync.controls().manual {
            return;
        }
        let ids = self.selected_ids();
        for id in ids {
            if let Some(state) = self.states.get_mut(&id) {
                state.mode = ColorMode::Temperature;
                state.temperature = Some(value);
            } else {
                self.states.insert(
                    id.clone(),
                    BulbState {
                        mode: ColorMode::Temperature,
                        temperature: Some(value),
                        ..Default::default()
                    },
                );
            }
            self.edits.touch(&id);
            self.dashboard.pending_temperature.insert(id, value);
        }
        self.mark_pending();
    }

    pub(crate) fn mark_pending(&mut self) {
        self.dashboard.due_at = Some(Instant::now() + Duration::from_millis(160));
    }

    pub(crate) fn flush_pending(&mut self) {
        for (id, command) in pending_commands(&mut self.dashboard) {
            self.send(&id, command);
        }
    }

    pub(crate) fn set_local_power(&mut self, id: &str, on: bool) {
        self.edits.touch(id);
        if let Some(state) = self.states.get_mut(id) {
            state.power = Some(on);
        } else {
            self.states.insert(
                id.to_owned(),
                BulbState {
                    power: Some(on),
                    ..Default::default()
                },
            );
        }
    }

    pub(crate) fn send(&mut self, id: &str, command: Command) {
        let name = self
            .bulbs
            .iter()
            .find(|bulb| bulb.id == id)
            .map_or_else(|| "the bulb".to_owned(), |bulb| bulb.name.clone());
        let Some(handle) = self.workers.get(id) else {
            self.notify(
                format!("No connection is open to {name}."),
                ToastKind::Error,
            );
            return;
        };
        if handle.send(command).is_err() {
            self.workers.remove(id);
            let in_run = self.sync.status.is_running();
            if self.health.apply(id, HealthEvent::Exited, in_run) {
                self.notify(
                    format!("The connection to {name} stopped."),
                    ToastKind::Error,
                );
            }
            self.configure_sync();
        }
    }

    pub(crate) fn open_add(&mut self) {
        self.leave_color();
        if self.startup_error.is_some() {
            self.notify(
                "Fix or remove the unreadable config file before adding a bulb.",
                ToastKind::Error,
            );
            return;
        }
        if !self.ensure_sync_stopped("add a bulb") {
            return;
        }
        self.form = FormState::default();
        self.set_screen(Screen::Form(FormMode::Add));
    }

    pub(crate) fn open_edit(&mut self, id: &str) {
        self.leave_color();
        if !self.ensure_sync_stopped("edit a bulb") {
            return;
        }
        let Some(bulb) = self.bulbs.iter().find(|bulb| bulb.id == id) else {
            return;
        };
        self.form = FormState {
            name: TextEdit::new(bulb.name.clone()),
            address: TextEdit::new(bulb.address.clone()),
            model: TextEdit::new({
                let empty = String::new();
                bulb.model.clone().unwrap_or(empty)
            }),
            ..FormState::default()
        };
        self.set_screen(Screen::Form(FormMode::Edit(id.to_owned())));
    }

    pub(crate) fn edit_selected(&mut self) {
        match self.selection.clone() {
            Some(Selection::Bulb(id)) => self.open_edit(&id),
            Some(Selection::All) => {
                self.notify("Select one bulb to edit it.", ToastKind::Info);
            }
            None => {}
        }
    }

    pub(crate) fn save_form(&mut self) {
        if self.startup_error.is_some() {
            self.notify(
                "Fix or remove the unreadable config file before saving a bulb.",
                ToastKind::Error,
            );
            return;
        }
        if !self.ensure_sync_stopped("change a bulb") {
            return;
        }
        if self.demo {
            self.notify("Demo mode does not save bulbs.", ToastKind::Info);
            return;
        }
        let name = self.form.name.text.trim().to_owned();
        let address = self.form.address.text.trim().to_owned();
        let model = self.form.model.text.trim().to_owned();
        let token = self.form.token.text.trim().to_owned();
        let editing = match &self.screen {
            Screen::Form(FormMode::Edit(id)) => Some(id.clone()),
            _ => None,
        };
        let token_required = editing.is_none();
        let issues = validate_draft(&name, &address, &model, &token, token_required);
        self.form.errors.clear();
        if !issues.is_empty() {
            for issue in issues {
                self.form.errors.insert(issue.field, issue.message);
            }
            return;
        }

        match editing {
            None => {
                let bulb = Bulb {
                    id: new_id(),
                    name,
                    address,
                    model: trim_model(&model),
                };
                let bulbs = match self.registry.add(&self.bulbs, &bulb, &token) {
                    Ok(bulbs) => bulbs,
                    Err(error) => {
                        self.show_save_error(error, &bulb);
                        return;
                    }
                };
                let before = self.bulbs.len();
                let id = bulb.id.clone();
                self.bulbs = bulbs;
                self.selection = Some(selection_after_add(before, &id, self.selection.as_ref()));
                self.spawn_worker(&bulb);
                self.set_screen(Screen::Dashboard);
                self.notify(format!("Added {}.", bulb.name), ToastKind::Success);
            }
            Some(id) => {
                let bulb = Bulb {
                    id,
                    name,
                    address,
                    model: trim_model(&model),
                };
                let new_token = (!token.is_empty()).then_some(token.as_str());
                let bulbs = match self.registry.update(&self.bulbs, &bulb, new_token) {
                    Ok(bulbs) => bulbs,
                    Err(error) => {
                        self.show_save_error(error, &bulb);
                        return;
                    }
                };
                self.bulbs = bulbs;
                self.spawn_worker(&bulb);
                self.set_screen(Screen::Dashboard);
                self.notify(format!("Saved {}.", bulb.name), ToastKind::Success);
            }
        }
    }

    fn show_save_error(&mut self, error: RegistryError, bulb: &Bulb) {
        match error {
            RegistryError::Invalid(BulbListIssue::DuplicateName { .. }) => {
                self.form.errors.insert(
                    BulbField::Name,
                    format!("A bulb named {} already exists.", bulb.name),
                );
            }
            RegistryError::Invalid(BulbListIssue::DuplicateAddress { .. }) => {
                self.form.errors.insert(
                    BulbField::Address,
                    format!("A bulb at {} already exists.", bulb.address),
                );
            }
            error => self.notify(error.to_string(), ToastKind::Error),
        }
    }

    pub(crate) fn open_remove(&mut self, id: &str) {
        self.leave_color();
        if !self.ensure_sync_stopped("remove a bulb") {
            return;
        }
        if self.bulbs.iter().any(|bulb| bulb.id == id) {
            self.set_screen(Screen::Remove(id.to_owned()));
        }
    }

    pub(crate) fn confirm_remove(&mut self) {
        if !self.ensure_sync_stopped("remove a bulb") {
            return;
        }
        if self.demo {
            self.notify("Demo mode does not change bulbs.", ToastKind::Info);
            return;
        }
        let Screen::Remove(id) = self.screen.clone() else {
            return;
        };
        let Some(bulb) = self.bulbs.iter().find(|bulb| bulb.id == id).cloned() else {
            self.set_screen(Screen::Dashboard);
            return;
        };
        let remaining = match self.registry.remove(&self.bulbs, &id) {
            Ok(bulbs) => bulbs,
            Err(error) => {
                self.notify(error.to_string(), ToastKind::Error);
                return;
            }
        };
        self.bulbs = remaining;
        self.states.remove(&id);
        self.health.remove(&id);
        self.edits.remove(&id);
        if let Some(handle) = self.workers.remove(&id) {
            handle.stop();
        }
        self.selection =
            crate::bulb::selection::resolve_selection(&self.bulbs, self.selection.as_ref());
        self.set_screen(Screen::Dashboard);
        self.notify(format!("Removed {}.", bulb.name), ToastKind::Success);
    }

    pub(crate) fn open_settings(&mut self) {
        self.leave_color();
        self.set_screen(Screen::Settings);
    }

    pub(crate) fn open_sync(&mut self) {
        self.leave_color();
        self.set_screen(Screen::Sync);
        let sent = self
            .screen_handle
            .as_ref()
            .is_some_and(|handle| handle.send(ScreenCommand::ListMonitors));
        if !sent {
            self.restart_screen();
        }
    }

    pub(crate) fn back(&mut self) {
        self.close_overlays();
        self.set_screen(Screen::Dashboard);
    }

    pub(crate) fn notify(&mut self, message: impl Into<String>, kind: ToastKind) {
        let toast = Toast {
            id: self.next_toast,
            message: message.into(),
            kind,
            created_at: Instant::now(),
        };
        self.next_toast += 1;
        self.toasts.push(toast);
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
        if let Some(waker) = &self.waker {
            waker();
        }
    }

    pub(crate) fn dismiss_toast(&mut self, id: u64) {
        self.toasts.retain(|toast| toast.id != id);
    }

    pub(crate) fn save_preferences(&mut self) {
        if self.demo {
            return;
        }
        if let Err(error) = self
            .preferences_store
            .save(crate::bulb::store::Preferences {
                sync_gain: self.sync.gain,
                colour_intensity: self.sync.colour_intensity,
                appearance: self.appearance,
            })
        {
            self.notify(error.to_string(), ToastKind::Error);
        }
    }

    pub(crate) fn schedule_preferences(&mut self) {
        if self.demo {
            return;
        }
        self.preferences_due = Some(Instant::now() + Duration::from_millis(400));
    }

    pub(crate) fn set_sync_gain(&mut self, gain: f32) {
        self.sync.gain = normalize_sync_gain(gain);
        self.configure_sync();
        self.schedule_preferences();
    }

    pub(crate) fn set_sync_intensity(&mut self, intensity: f32) {
        self.sync.colour_intensity = normalize_sync_intensity(intensity);
        if let Some(scene) = self.sync.last_scene {
            self.sync.derived = Some(intensify(scene.color, self.sync.colour_intensity));
        }
        self.configure_sync();
        self.schedule_preferences();
    }

    pub(crate) fn probe_unreachable(&mut self) {
        let mut gone = Vec::new();
        for id in self.health.unreachable_ids() {
            let Some(handle) = self.workers.get(&id) else {
                continue;
            };
            if handle.send(Command::Probe).is_err() {
                gone.push(id);
            }
        }
        let had_gone = !gone.is_empty();
        let in_run = self.sync.status.is_running();
        for id in gone {
            self.workers.remove(&id);
            self.health.apply(&id, HealthEvent::Exited, in_run);
        }
        if had_gone {
            self.configure_sync();
        }
    }

    pub(crate) fn shutdown(&mut self) {
        self.leave_color();
        self.flush_pending();
        if self.sync.status.is_active() {
            self.stop_sync();
        }
        while self.sync.status.is_stopping() {
            self.drain_events();
            self.advance_sync(Instant::now());
            if !self.sync.status.is_stopping() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        for handle in self.workers.values() {
            handle.stop();
        }
        self.screen_handle.take();
        self.preferences_due = None;
        self.save_preferences();
    }

    fn drain_events(&mut self) {
        let mut events = Vec::new();
        if let Some(handle) = &self.screen_handle {
            while let Ok(event) = handle.events.try_recv() {
                events.push(event);
            }
        }
        for event in events {
            self.handle_screen_event(event);
        }
        while let Ok(event) = self.worker_events.try_recv() {
            let _ = self.handle_worker_event(event);
        }
        let latest = self
            .screen_handle
            .as_ref()
            .and_then(|handle| handle.latest.lock().ok().and_then(|mut slot| slot.take()));
        if let Some(frame) = latest {
            self.note_sync_frame(frame);
        }
    }
}

fn pending_commands(dashboard: &mut Dashboard) -> Vec<(String, Command)> {
    dashboard.due_at = None;
    let mut commands = Vec::new();
    for (id, value) in std::mem::take(&mut dashboard.pending_brightness) {
        commands.push((id, Command::SetBrightness(value)));
    }
    for (id, value) in std::mem::take(&mut dashboard.pending_temperature) {
        commands.push((id, Command::SetTemperature(value)));
    }
    for (id, value) in std::mem::take(&mut dashboard.pending_color) {
        commands.push((id, Command::SetColor(value)));
    }
    commands
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::pending_commands;
    use crate::app::state::Dashboard;
    use crate::bulb::model::Rgb;
    use crate::bulb::worker::Command;

    #[test]
    fn the_last_pending_control_values_become_commands() {
        let mut dashboard = Dashboard::default();
        dashboard.pending_brightness.insert("desk".to_owned(), 40);
        dashboard
            .pending_temperature
            .insert("shelf".to_owned(), 3_000);
        dashboard
            .pending_color
            .insert("ceiling".to_owned(), Rgb::new(1, 2, 3));
        dashboard.due_at = Some(Instant::now() + Duration::from_millis(160));

        let commands = pending_commands(&mut dashboard);

        assert_eq!(commands.len(), 3);
        let mut brightness = false;
        let mut temperature = false;
        let mut color = false;
        for (id, command) in commands {
            match command {
                Command::SetBrightness(value) => {
                    assert_eq!(id, "desk");
                    assert_eq!(value, 40);
                    brightness = true;
                }
                Command::SetTemperature(value) => {
                    assert_eq!(id, "shelf");
                    assert_eq!(value, 3_000);
                    temperature = true;
                }
                Command::SetColor(value) => {
                    assert_eq!(id, "ceiling");
                    assert_eq!(value, Rgb::new(1, 2, 3));
                    color = true;
                }
                _ => {}
            }
        }
        assert!(brightness && temperature && color);
        assert!(dashboard.due_at.is_none());
        assert!(dashboard.pending_brightness.is_empty());
        assert!(dashboard.pending_temperature.is_empty());
        assert!(dashboard.pending_color.is_empty());
    }
}
