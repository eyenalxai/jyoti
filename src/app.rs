mod actions;
pub(crate) mod state;
mod sync;

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use rune::{Key, KeyEvent, Tx, Waker};
use rune_kit::shell::PaneState;

use crate::app::state::{
    ColorState, Dashboard, FormState, Screen, SyncState, Toast, ToastKind, toast_duration,
};
use crate::bulb::keyring::KeyringStore;
use crate::bulb::model::{Bulb, BulbState, ColorMode, Rgb};
use crate::bulb::selection::{Selection, resolve_selection};
use crate::bulb::store::{BulbStore, PreferencesStore};
use crate::bulb::worker::{Command, WorkerEvent};
use crate::config::AppConfig;
use crate::screen::capture::{ScreenCommand, ScreenHandle};

pub struct App {
    pub config: AppConfig,
    pub store: BulbStore,
    pub preferences_store: PreferencesStore,
    pub keyring: KeyringStore,
    pub bulbs: Vec<Bulb>,
    pub states: HashMap<String, BulbState>,
    pub errors: HashMap<String, String>,
    pub selection: Option<Selection>,
    pub screen: Screen,
    pub dashboard: Dashboard,
    pub form: FormState,
    pub color: ColorState,
    pub sync: SyncState,
    pub toasts: Vec<Toast>,
    pub next_toast: u64,
    pub workers: HashMap<String, Sender<Command>>,
    pub worker_events: Receiver<WorkerEvent>,
    pub worker_sender: Sender<WorkerEvent>,
    pub screen_handle: Option<ScreenHandle>,
    pub waker: Option<Waker>,
    pub sidebar: PaneState,
    pub started: bool,
    pub startup_error: Option<String>,
    pub demo: bool,
    pub capture_once: bool,
}

impl App {
    pub fn new() -> Self {
        let config = AppConfig::from_env();
        let store = BulbStore::new(config.config_path.clone());
        let preferences_store = PreferencesStore::new(config.preferences_path.clone());
        let keyring = KeyringStore::new(config.keyring_service.clone());
        let preferences = preferences_store.load();
        let (worker_sender, worker_events) = mpsc::channel();

        let demo = rune::extra_args()
            .iter()
            .any(|arg| arg == "demo" || arg == "--demo");
        let capture_once = rune::extra_args()
            .iter()
            .any(|arg| arg == "capture-once" || arg == "--capture-once");
        let (bulbs, startup_error) = if demo {
            (demo_bulbs(), None)
        } else {
            match store.load() {
                Ok(bulbs) => (bulbs, None),
                Err(error) => (Vec::new(), Some(error.to_string())),
            }
        };
        let states = if demo { demo_states() } else { HashMap::new() };
        let selection = if demo {
            Some(Selection::All)
        } else {
            resolve_selection(&bulbs, None)
        };
        let sync = SyncState {
            gain: if demo { 1.0 } else { preferences.sync_gain },
            ..SyncState::default()
        };

        Self {
            config,
            store,
            preferences_store,
            keyring,
            bulbs,
            states,
            errors: HashMap::new(),
            selection,
            screen: Screen::Dashboard,
            dashboard: Dashboard::default(),
            form: FormState::default(),
            color: ColorState::default(),
            sync,
            toasts: Vec::new(),
            next_toast: 1,
            workers: HashMap::new(),
            worker_events,
            worker_sender,
            screen_handle: None,
            waker: None,
            sidebar: PaneState::new(276.0).range(200.0, 420.0),
            started: false,
            startup_error,
            demo,
            capture_once,
        }
    }

    pub fn start(&mut self, waker: Waker) {
        if self.started {
            return;
        }
        self.started = true;
        self.waker = Some(std::sync::Arc::clone(&waker));
        if self.demo {
            return;
        }
        if let Some(error) = self.startup_error.as_ref() {
            self.notify(error.clone(), ToastKind::Error);
        }
        let bulbs = self.bulbs.clone();
        for bulb in bulbs {
            self.spawn_worker(&bulb);
        }
        let handle = ScreenHandle::start(waker);
        let _ = handle.send(ScreenCommand::ListMonitors);
        if self.capture_once {
            let _ = handle.send(ScreenCommand::CaptureOnce);
        }
        self.screen_handle = Some(handle);
    }

    pub fn poll(&mut self, _dt: f64, tx: &mut Tx) -> bool {
        let mut changed = false;

        while let Ok(event) = self.worker_events.try_recv() {
            changed |= self.handle_worker_event(event);
        }

        let mut screen_events = Vec::new();
        if let Some(handle) = &self.screen_handle {
            while let Ok(event) = handle.events.try_recv() {
                screen_events.push(event);
            }
        }
        for event in screen_events {
            self.handle_screen_event(event);
            changed = true;
        }

        let now = Instant::now();
        if let Some(due) = self.dashboard.due_at {
            if now >= due {
                self.flush_pending();
                changed = true;
            } else {
                tx.wake_after(due - now);
            }
        }

        let before = self.toasts.len();
        self.toasts
            .retain(|toast| now.duration_since(toast.created_at) < toast_duration(toast.kind));
        if self.toasts.len() != before {
            changed = true;
        }
        if let Some(next) = self
            .toasts
            .iter()
            .map(|toast| toast.created_at + toast_duration(toast.kind))
            .min()
        {
            tx.wake_after(next.saturating_duration_since(now));
        }

        if self.sync.status.is_running() {
            if let Some(last) = self.sync.last_scene_at
                && last.elapsed() > Duration::from_secs(5)
                && !self.sync.stale
            {
                self.sync.stale = true;
            }
            if self.sync.last_probe.elapsed() > Duration::from_secs(5) {
                self.probe_unreachable();
                self.sync.last_probe = now;
            }
            tx.wake_after(Duration::from_secs(1));
            if self.screen == Screen::Sync {
                changed = true;
            }
        }

        if self.sync.pending_stop {
            if self
                .sync
                .restore_deadline
                .is_some_and(|deadline| now >= deadline)
            {
                self.finish_restore();
                changed = true;
            } else {
                tx.wake_after(Duration::from_millis(100));
            }
        }

        changed
    }

    pub fn on_key(&mut self, event: &KeyEvent, tx: &mut Tx) -> bool {
        if event.is_command('n') {
            self.open_add();
            return true;
        }
        if event.is_command('r') {
            self.refresh_all(tx);
            return true;
        }
        if event.is_command(',') {
            self.open_settings();
            return true;
        }
        if event.is_command('s') {
            self.open_sync();
            return true;
        }
        if event.is_command('e') {
            self.edit_selected();
            return true;
        }
        if event.is_command('q') {
            tx.quit();
            return true;
        }
        if event.key == Key::Escape {
            self.back();
            return true;
        }
        match &event.key {
            Key::Text(text) if text == " " => {
                self.toggle_power();
                true
            }
            Key::Text(text) if text.eq_ignore_ascii_case("w") => {
                self.set_mode_white();
                true
            }
            Key::Text(text) if text.eq_ignore_ascii_case("c") => {
                self.set_mode_color();
                true
            }
            Key::Text(text) if text == "?" => {
                self.screen = Screen::Help;
                true
            }
            _ => false,
        }
    }

    pub fn quit(&mut self, _tx: &mut Tx) {
        self.shutdown();
    }

    pub fn aggregate(&self) -> crate::bulb::aggregate::AggregateState {
        crate::bulb::aggregate::aggregate_state(&self.bulbs, &self.states, &self.errors)
    }

    pub fn selected_ids(&self) -> Vec<String> {
        match &self.selection {
            Some(Selection::All) => self.bulbs.iter().map(|bulb| bulb.id.clone()).collect(),
            Some(Selection::Bulb(id)) => vec![id.clone()],
            None => Vec::new(),
        }
    }

    pub fn selected_bulb(&self) -> Option<&Bulb> {
        match &self.selection {
            Some(Selection::Bulb(id)) => self.bulbs.iter().find(|bulb| &bulb.id == id),
            _ => None,
        }
    }

    pub fn selected_state(&self) -> Option<&BulbState> {
        self.selected_bulb()
            .and_then(|bulb| self.states.get(&bulb.id))
    }

    pub fn selection_label(&self) -> String {
        match &self.selection {
            Some(Selection::All) => "All bulbs".to_owned(),
            Some(Selection::Bulb(id)) => self
                .bulbs
                .iter()
                .find(|bulb| &bulb.id == id)
                .map_or_else(|| "jyoti".to_owned(), |bulb| bulb.name.clone()),
            None => "jyoti".to_owned(),
        }
    }
}

fn demo_bulbs() -> Vec<Bulb> {
    [
        ("Desk lamp", "192.168.1.52", "yeelink.light.color3"),
        ("Shelf lamp", "192.168.1.142", "yeelink.light.color3"),
        ("Ceiling", "192.168.1.132", "yeelink.light.color3"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (name, address, model))| Bulb {
        id: format!("00000000-0000-4000-8000-{:012}", index + 1),
        name: name.to_owned(),
        address: address.to_owned(),
        model: Some(model.to_owned()),
    })
    .collect()
}

fn demo_states() -> HashMap<String, BulbState> {
    let bulbs = demo_bulbs();
    bulbs
        .into_iter()
        .enumerate()
        .map(|(index, bulb)| {
            let state = BulbState {
                power: Some(true),
                brightness: Some(80 - index as u8 * 15),
                temperature: Some(3_000 + index as u16 * 700),
                color: Some(Rgb::new(255, 180 - index as u8 * 20, 120)),
                mode: ColorMode::Temperature,
            };
            (bulb.id, state)
        })
        .collect()
}
