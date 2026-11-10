mod actions;
mod color;
pub(crate) mod selected;
mod selection;
pub(crate) mod state;
pub(crate) mod sync;
pub(crate) mod view;

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use rune::{TextEdit, Tx, Waker};

use crate::app::selected::{SelectedLight, selected_light};
use crate::app::state::{Dashboard, LocalEdits, SyncState, Toast, ToastKind, toast_duration};
use crate::app::view::{Navigation, View, nav_direction};
use crate::bulb::health::HealthSet;
use crate::bulb::keyring::KeyringStore;
use crate::bulb::model::{Bulb, BulbState, ColorMode, Rgb};
use crate::bulb::registry::BulbRegistry;
use crate::bulb::selection::{Selection, resolve_selection};
use crate::bulb::store::{Appearance, BulbStore, PreferencesStore};
use crate::bulb::worker::{WorkerEvent, WorkerHandle};
use crate::config::AppConfig;
use crate::screen::capture::{ScreenCommand, ScreenHandle};

pub struct App {
    pub config: AppConfig,
    pub registry: BulbRegistry<BulbStore, KeyringStore>,
    pub preferences_store: PreferencesStore,
    pub bulbs: Vec<Bulb>,
    pub states: HashMap<String, BulbState>,
    pub health: HealthSet,
    pub edits: LocalEdits,
    pub selection: Option<Selection>,
    pub view: View,
    pub navigation: Navigation,
    pub appearance: Appearance,
    pub dashboard: Dashboard,
    pub sync: SyncState,
    pub toasts: Vec<Toast>,
    pub next_toast: u64,
    pub workers: HashMap<String, WorkerHandle>,
    pub worker_events: Receiver<WorkerEvent>,
    pub worker_sender: Sender<WorkerEvent>,
    pub screen_handle: Option<ScreenHandle>,
    pub waker: Option<Waker>,
    pub preferences_due: Option<Instant>,
    // The text inputs bind to a `&mut TextEdit`; off their view a shared
    // scratch buffer serves the binding.
    pub empty_edit: TextEdit,
    pub started: bool,
    pub startup_error: Option<String>,
    pub demo: bool,
    pub capture_once: bool,
}

impl App {
    pub fn new() -> Self {
        let config = AppConfig::from_env();
        let registry = BulbRegistry::new(
            BulbStore::new(config.config_path.clone()),
            KeyringStore::new(config.keyring_service.clone()),
        );
        let preferences_store = PreferencesStore::new(config.preferences_path.clone());
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
            match registry.load() {
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
            colour_intensity: if demo {
                35.0
            } else {
                preferences.colour_intensity
            },
            ..SyncState::default()
        };
        let mut app = Self {
            config,
            registry,
            preferences_store,
            bulbs,
            states,
            health: HealthSet::default(),
            edits: LocalEdits::default(),
            selection,
            view: View::control(),
            navigation: Navigation::default(),
            appearance: if demo {
                Appearance::System
            } else {
                preferences.appearance
            },
            dashboard: Dashboard::default(),
            sync,
            toasts: Vec::new(),
            next_toast: 1,
            workers: HashMap::new(),
            worker_events,
            worker_sender,
            screen_handle: None,
            waker: None,
            preferences_due: None,
            empty_edit: TextEdit::new(""),
            started: false,
            startup_error,
            demo,
            capture_once,
        };
        if demo {
            let ids: Vec<String> = app.states.keys().cloned().collect();
            for id in ids {
                app.health
                    .apply(&id, crate::bulb::health::HealthEvent::State, false);
            }
        }
        app
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
        let mut latest = None;
        if let Some(handle) = &self.screen_handle {
            while let Ok(event) = handle.events.try_recv() {
                screen_events.push(event);
            }
            latest = handle.latest.lock().ok().and_then(|mut slot| slot.take());
        }
        for event in screen_events {
            self.handle_screen_event(event);
            changed = true;
        }
        if let Some(frame) = latest {
            self.note_sync_frame(frame);
            changed = true;
        }

        let now = Instant::now();
        if let Some((view, due)) = self.navigation.pending.take() {
            if now >= due {
                self.apply_view(view);
                self.navigation.entering_until = Some(now + Duration::from_millis(16));
                changed = true;
            } else {
                self.navigation.pending = Some((view, due));
                tx.wake_after(due - now);
                changed = true;
            }
        }
        if self
            .navigation
            .entering_until
            .is_some_and(|until| now >= until)
        {
            self.navigation.entering_until = None;
        }
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

        if let Some(due) = self.preferences_due {
            if now >= due {
                self.save_preferences();
                self.preferences_due = None;
            } else {
                tx.wake_after(due - now);
            }
        }

        if self.sync.status.is_in_run() {
            if self.sync.last_probe.elapsed() > Duration::from_secs(5) {
                self.probe_unreachable();
                self.sync.last_probe = now;
            }
            tx.wake_after(Duration::from_secs(1));
            if matches!(self.view, View::Sync(_)) {
                changed = true;
            }
        }

        changed |= self.advance_sync(now);
        if self.sync.status.is_stopping() {
            tx.wake_after(Duration::from_millis(100));
        }

        changed
    }

    pub fn quit(&mut self, _tx: &mut Tx) {
        self.shutdown();
    }

    pub fn aggregate(&self) -> crate::bulb::aggregate::AggregateState {
        crate::bulb::aggregate::aggregate_state(&self.bulbs, &self.states, &self.health)
    }

    pub fn selected_light(&self) -> Option<SelectedLight> {
        selected_light(
            &self.bulbs,
            &self.states,
            &self.health,
            self.selection.as_ref(),
        )
    }

    pub fn selected_ids(&self) -> Vec<String> {
        match &self.selection {
            Some(Selection::All) => self.bulbs.iter().map(|bulb| bulb.id.clone()).collect(),
            Some(Selection::Bulb(id)) => vec![id.clone()],
            None => Vec::new(),
        }
    }

    pub(crate) fn show_now(&mut self, view: View) {
        self.navigation.pending = None;
        self.navigation.entering_until = None;
        self.apply_view(view);
    }

    pub(crate) fn show_pending(&mut self) {
        self.navigation.entering_until = None;
        if let Some((view, _)) = self.navigation.pending.take() {
            self.apply_view(view);
        }
    }

    fn apply_view(&mut self, view: View) {
        self.cancel_color();
        self.view.close_transients();
        self.view.navigate(view);
    }

    /// The one navigation transition. The Control overlay, colour editor and
    /// Sync overlay close before the destination renders; the editor reverts
    /// to its pre-open values.
    pub(crate) fn navigate(&mut self, next: View) {
        self.cancel_color();
        self.view.close_transients();
        if self.view.same_place(&next) {
            self.apply_view(next);
            self.navigation.pending = None;
            self.navigation.entering_until = None;
            return;
        }
        self.navigation.direction = nav_direction(&self.view, &next);
        self.navigation.pending = Some((next, Instant::now() + Duration::from_millis(16)));
        if let Some(waker) = &self.waker {
            waker();
        }
    }

    pub(crate) fn set_appearance(&mut self, appearance: Appearance, tx: &mut Tx) {
        if self.appearance == appearance {
            return;
        }
        self.appearance = appearance;
        self.schedule_preferences();
        tx.instant();
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
