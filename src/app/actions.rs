use std::time::{Duration, Instant};

use rune::{TextEdit, Tx};

use crate::app::App;
use crate::app::state::{FormMode, FormState, Screen, Toast, ToastKind};
use crate::bulb::aggregate::AggregatePower;
use crate::bulb::model::{
    Bulb, BulbField, BulbState, ColorMode, NEUTRAL_TEMPERATURE, Rgb, new_id, trim_model,
    validate_draft,
};
use crate::bulb::selection::{Selection, resolve_selection, selection_after_add};
use crate::bulb::worker::{Command, WorkerEvent};
use crate::screen::capture::ScreenCommand;
use crate::screen::gain::normalize_sync_gain;

impl App {
    fn ensure_sync_stopped(&mut self, action: &str) -> bool {
        if self.sync.status.is_busy() {
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
            let _ = previous.send(Command::Stop);
        }
        let Some(waker) = self.waker.clone() else {
            return;
        };
        let sender = crate::bulb::worker::spawn(
            bulb.clone(),
            self.keyring.clone(),
            self.config.clone(),
            self.worker_sender.clone(),
            waker,
        );
        self.workers.insert(bulb.id.clone(), sender);
        self.errors.remove(&bulb.id);
    }

    pub(crate) fn handle_worker_event(&mut self, event: WorkerEvent) -> bool {
        match event {
            WorkerEvent::State { id, state } => {
                let was_on = self.states.get(&id).and_then(|state| state.power) == Some(true);
                let is_on = state.power == Some(true);
                self.states.insert(id.clone(), state);
                self.errors.remove(&id);
                self.sync.failures.remove(&id);
                if self.sync.status.is_running() {
                    self.sync.unreachable.remove(&id);
                    if !was_on && is_on {
                        self.republish_to(&id);
                    }
                }
                true
            }
            WorkerEvent::Sent { id } => {
                let changed = self.sync.unreachable.remove(&id);
                let had_error = self.errors.remove(&id).is_some();
                self.sync.failures.remove(&id);
                changed || had_error
            }
            WorkerEvent::Error { id, error } => {
                self.errors.insert(id.clone(), error.to_string());
                if self.sync.status.is_running() {
                    let failures = self.sync.failures.entry(id.clone()).or_insert(0);
                    *failures = failures.saturating_add(1);
                    if *failures >= 3 {
                        self.sync.unreachable.insert(id.clone());
                    }
                }
                self.notify(error.to_string(), ToastKind::Error);
                true
            }
            WorkerEvent::Restored { id, ok } => {
                self.sync.pending_restore.remove(&id);
                let report = self
                    .sync
                    .restore_report
                    .get_or_insert_with(Default::default);
                if ok {
                    report.restored += 1;
                } else if let Some(bulb) = self.bulbs.iter().find(|bulb| bulb.id == id) {
                    report.not_restored.push(bulb.name.clone());
                }
                if self.sync.pending_restore.is_empty() {
                    self.finish_restore();
                }
                true
            }
            WorkerEvent::Reachable { id, reachable } => {
                if reachable {
                    let changed = self.sync.unreachable.remove(&id);
                    let had_error = self.errors.remove(&id).is_some();
                    self.republish_to(&id);
                    changed || had_error
                } else {
                    self.sync.unreachable.insert(id)
                }
            }
        }
    }

    pub(crate) fn refresh_all(&mut self, tx: &mut Tx) {
        match self.store.load() {
            Ok(bulbs) => {
                self.startup_error = None;
                let ids: Vec<String> = self.workers.keys().cloned().collect();
                for id in ids {
                    if let Some(sender) = self.workers.remove(&id) {
                        let _ = sender.send(Command::Stop);
                    }
                }
                self.bulbs = bulbs;
                let keep: std::collections::HashSet<String> =
                    self.bulbs.iter().map(|bulb| bulb.id.clone()).collect();
                self.states.retain(|id, _| keep.contains(id));
                self.errors.clear();
                self.sync.unreachable.clear();
                self.sync.failures.clear();
                self.selection = resolve_selection(&self.bulbs, self.selection.as_ref());
                let bulbs = self.bulbs.clone();
                for bulb in bulbs {
                    self.spawn_worker(&bulb);
                }
                if self.sync.status.is_running() {
                    let ids: Vec<String> = self.bulbs.iter().map(|bulb| bulb.id.clone()).collect();
                    for id in ids {
                        self.republish_to(&id);
                    }
                }
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
        if target {
            for id in ids {
                self.republish_to(&id);
            }
        }
    }

    pub(crate) fn set_mode_white(&mut self) {
        if self.sync.status.is_running() {
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
            self.send(&id, Command::SetTemperature(temperature));
        }
    }

    pub(crate) fn set_mode_color(&mut self) {
        if self.sync.status.is_running() {
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
            self.send(&id, Command::SetColor(color));
        }
    }

    pub(crate) fn set_brightness(&mut self, value: u8) {
        if self.sync.status.is_busy() {
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
            self.dashboard.pending_brightness.insert(id, value);
        }
        self.mark_pending();
    }

    pub(crate) fn set_temperature(&mut self, value: u16) {
        if self.sync.status.is_busy() {
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
            self.dashboard.pending_temperature.insert(id, value);
        }
        self.mark_pending();
    }

    pub(crate) fn mark_pending(&mut self) {
        self.dashboard.due_at = Some(Instant::now() + Duration::from_millis(160));
    }

    pub(crate) fn flush_pending(&mut self) {
        self.dashboard.due_at = None;
        let brightness = std::mem::take(&mut self.dashboard.pending_brightness);
        for (id, value) in brightness {
            self.send(&id, Command::SetBrightness(value));
        }
        let temperature = std::mem::take(&mut self.dashboard.pending_temperature);
        for (id, value) in temperature {
            self.send(&id, Command::SetTemperature(value));
        }
        let color = std::mem::take(&mut self.dashboard.pending_color);
        for (id, value) in color {
            self.send(&id, Command::SetColor(value));
        }
    }

    pub(crate) fn set_local_power(&mut self, id: &str, on: bool) {
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
        let Some(sender) = self.workers.get(id) else {
            let name = self
                .bulbs
                .iter()
                .find(|bulb| bulb.id == id)
                .map_or_else(|| "the bulb".to_owned(), |bulb| bulb.name.clone());
            self.notify(
                format!("No connection is open to {name}."),
                ToastKind::Error,
            );
            return;
        };
        let _ = sender.send(command);
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
        self.screen = Screen::Form(FormMode::Add);
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
        self.screen = Screen::Form(FormMode::Edit(id.to_owned()));
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

        if self
            .bulbs
            .iter()
            .any(|bulb| Some(&bulb.id) != editing.as_ref() && bulb.name.eq_ignore_ascii_case(&name))
        {
            self.form.errors.insert(
                BulbField::Name,
                format!("A bulb named {name} already exists."),
            );
            return;
        }
        if self
            .bulbs
            .iter()
            .any(|bulb| Some(&bulb.id) != editing.as_ref() && bulb.address == address)
        {
            self.form.errors.insert(
                BulbField::Address,
                format!("A bulb at {address} already exists."),
            );
            return;
        }

        match editing {
            None => {
                let id = new_id();
                if let Err(error) = self.keyring.set(&id, &token) {
                    self.notify(error.to_string(), ToastKind::Error);
                    return;
                }
                let added_name = name.clone();
                let bulb = Bulb {
                    id: id.clone(),
                    name,
                    address,
                    model: trim_model(&model),
                };
                let mut bulbs = self.bulbs.clone();
                bulbs.push(bulb.clone());
                if let Err(error) = self.store.save(&bulbs) {
                    let _ = self.keyring.remove(&id);
                    self.notify(error.to_string(), ToastKind::Error);
                    return;
                }
                let before = self.bulbs.len();
                self.bulbs = bulbs;
                self.selection = Some(selection_after_add(before, &id, self.selection.as_ref()));
                self.spawn_worker(&bulb);
                self.screen = Screen::Dashboard;
                self.notify(format!("Added {added_name}."), ToastKind::Success);
            }
            Some(id) => {
                let Some(index) = self.bulbs.iter().position(|bulb| bulb.id == id) else {
                    self.notify("That bulb is no longer in the list.", ToastKind::Error);
                    return;
                };
                let previous_token = if token.is_empty() {
                    None
                } else {
                    self.keyring.get(&id).ok().flatten()
                };
                if !token.is_empty()
                    && let Err(error) = self.keyring.set(&id, &token)
                {
                    self.notify(error.to_string(), ToastKind::Error);
                    return;
                }
                let bulb = Bulb {
                    id: id.clone(),
                    name,
                    address,
                    model: trim_model(&model),
                };
                let mut bulbs = self.bulbs.clone();
                if let Some(slot) = bulbs.get_mut(index) {
                    *slot = bulb.clone();
                }
                if let Err(error) = self.store.save(&bulbs) {
                    if !token.is_empty() {
                        match previous_token {
                            Some(previous) => {
                                let _ = self.keyring.set(&id, &previous);
                            }
                            None => {
                                let _ = self.keyring.remove(&id);
                            }
                        }
                    }
                    self.notify(error.to_string(), ToastKind::Error);
                    return;
                }
                self.bulbs = bulbs;
                self.spawn_worker(&bulb);
                self.screen = Screen::Dashboard;
                self.notify(format!("Saved {}.", bulb.name), ToastKind::Success);
            }
        }
    }

    pub(crate) fn open_remove(&mut self, id: &str) {
        self.leave_color();
        if !self.ensure_sync_stopped("remove a bulb") {
            return;
        }
        if self.bulbs.iter().any(|bulb| bulb.id == id) {
            self.screen = Screen::Remove(id.to_owned());
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
            self.screen = Screen::Dashboard;
            return;
        };
        let remaining: Vec<Bulb> = self
            .bulbs
            .iter()
            .filter(|bulb| bulb.id != id)
            .cloned()
            .collect();
        let previous_token = self.keyring.get(&id).ok().flatten();
        if let Err(error) = self.keyring.remove(&id) {
            self.notify(error.to_string(), ToastKind::Error);
            return;
        }
        if let Err(error) = self.store.save(&remaining) {
            if let Some(previous) = previous_token {
                let _ = self.keyring.set(&id, &previous);
            }
            self.notify(error.to_string(), ToastKind::Error);
            return;
        }
        self.bulbs = remaining;
        self.states.remove(&id);
        self.errors.remove(&id);
        if let Some(sender) = self.workers.remove(&id) {
            let _ = sender.send(Command::Stop);
        }
        self.selection =
            crate::bulb::selection::resolve_selection(&self.bulbs, self.selection.as_ref());
        self.screen = Screen::Dashboard;
        self.notify(format!("Removed {}.", bulb.name), ToastKind::Success);
    }

    pub(crate) fn open_settings(&mut self) {
        self.leave_color();
        self.screen = Screen::Settings;
    }

    pub(crate) fn open_sync(&mut self) {
        self.leave_color();
        self.screen = Screen::Sync;
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
        if self.screen == Screen::Color {
            self.cancel_color();
        } else {
            self.screen = Screen::Dashboard;
        }
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
            })
        {
            self.notify(error.to_string(), ToastKind::Error);
        }
    }

    pub(crate) fn set_sync_gain(&mut self, gain: f32) {
        self.sync.gain = normalize_sync_gain(gain);
        self.save_preferences();
    }

    pub(crate) fn probe_unreachable(&mut self) {
        let ids: Vec<String> = self.sync.unreachable.iter().cloned().collect();
        for id in ids {
            self.send(&id, Command::Probe);
        }
    }

    pub(crate) fn shutdown(&mut self) {
        self.leave_color();
        if self.sync.status.is_active() {
            self.begin_restore();
            let deadline = Instant::now() + Duration::from_millis(2_200);
            while Instant::now() < deadline && !self.sync.pending_restore.is_empty() {
                while let Ok(event) = self.worker_events.try_recv() {
                    let _ = self.handle_worker_event(event);
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            self.finish_restore();
        }
        for sender in self.workers.values() {
            let _ = sender.send(Command::Stop);
        }
        self.screen_handle.take();
        self.save_preferences();
    }

    pub(crate) fn single_state(&self) -> BulbState {
        let fallback = BulbState::default();
        self.selected_state().cloned().unwrap_or(fallback)
    }
}
