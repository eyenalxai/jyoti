use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use rune::TextEdit;

use crate::app::sync::run::{Controls, RunPhase, STALE_AFTER};
use crate::bulb::model::{BulbField, BulbState, Rgb};
use crate::screen::monitor::{MonitorInfo, Scene};
use crate::screen::sync::RestoreReport;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Screen {
    Dashboard,
    Sync,
    Settings,
    Form(FormMode),
    Remove(String),
}

impl Screen {
    pub fn element_key(&self) -> &'static str {
        match self {
            Screen::Dashboard => "dashboard",
            Screen::Sync => "sync",
            Screen::Settings => "settings",
            Screen::Form(_) => "form",
            Screen::Remove(_) => "remove",
        }
    }

    fn rank(&self) -> u8 {
        match self {
            Screen::Dashboard => 0,
            Screen::Sync => 1,
            Screen::Settings => 2,
            Screen::Form(_) | Screen::Remove(_) => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavDirection {
    #[default]
    Forward,
    Backward,
}

#[derive(Default)]
pub struct Navigation {
    pub direction: NavDirection,
    pub pending: Option<(Screen, Instant)>,
    pub entering_until: Option<Instant>,
}

pub fn nav_direction(from: &Screen, to: &Screen) -> NavDirection {
    if matches!(from, Screen::Form(_) | Screen::Remove(_)) {
        if matches!(to, Screen::Dashboard) {
            NavDirection::Backward
        } else {
            NavDirection::Forward
        }
    } else if matches!(to, Screen::Form(_) | Screen::Remove(_)) {
        NavDirection::Forward
    } else if to.rank() < from.rank() {
        NavDirection::Backward
    } else {
        NavDirection::Forward
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormMode {
    Add,
    Edit(String),
}

pub struct FormState {
    pub name: TextEdit,
    pub address: TextEdit,
    pub model: TextEdit,
    pub token: TextEdit,
    pub reveal: bool,
    pub errors: HashMap<BulbField, String>,
}

impl Default for FormState {
    fn default() -> Self {
        Self {
            name: TextEdit::new(""),
            address: TextEdit::new(""),
            model: TextEdit::new(""),
            token: TextEdit::new(""),
            reveal: false,
            errors: HashMap::new(),
        }
    }
}

pub struct ColorState {
    pub open: bool,
    pub targets: Vec<String>,
    pub hue: f32,
    pub saturation: f32,
    pub value: f32,
    pub hex: TextEdit,
    pub preview: Rgb,
    pub error: Option<String>,
    pub revert: HashMap<String, BulbState>,
}

impl Default for ColorState {
    fn default() -> Self {
        Self {
            open: false,
            targets: Vec::new(),
            hue: 0.0,
            saturation: 0.0,
            value: 100.0,
            hex: TextEdit::new(""),
            preview: Rgb::WHITE,
            error: None,
            revert: HashMap::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Error,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub id: u64,
    pub message: String,
    pub kind: ToastKind,
    pub created_at: Instant,
}

#[derive(Default)]
pub struct Dashboard {
    pub pending_brightness: HashMap<String, u8>,
    pub pending_temperature: HashMap<String, u16>,
    pub pending_color: HashMap<String, Rgb>,
    pub due_at: Option<Instant>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StopPhase {
    #[default]
    Idle,
    Waiting {
        until: Instant,
    },
    Restoring {
        deadline: Instant,
    },
}

pub struct SyncState {
    pub status: RunPhase,
    pub next_generation: u64,
    pub monitors: Vec<MonitorInfo>,
    pub capture_error: Option<String>,
    pub selected_monitor: Option<String>,
    pub gain: f32,
    pub colour_intensity: f32,
    pub last_scene: Option<Scene>,
    pub last_scene_at: Option<Instant>,
    pub derived: Option<Rgb>,
    pub brightness: u8,
    pub flash_until: Option<Instant>,
    pub snapshots: HashMap<String, BulbState>,
    pub drove_brightness: HashSet<String>,
    pub restore_report: Option<RestoreReport>,
    pub unreachable: HashSet<String>,
    pub failures: HashMap<String, u8>,
    pub last_probe: Instant,
    pub pending_restore: HashSet<String>,
    pub stop: StopPhase,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            status: RunPhase::Idle,
            next_generation: 1,
            monitors: Vec::new(),
            capture_error: None,
            selected_monitor: None,
            gain: crate::screen::gain::SYNC_GAIN_DEFAULT,
            colour_intensity: crate::screen::intensity::SYNC_INTENSITY_DEFAULT,
            last_scene: None,
            last_scene_at: None,
            derived: None,
            brightness: 0,
            flash_until: None,
            snapshots: HashMap::new(),
            drove_brightness: HashSet::new(),
            restore_report: None,
            unreachable: HashSet::new(),
            failures: HashMap::new(),
            last_probe: Instant::now(),
            pending_restore: HashSet::new(),
            stop: StopPhase::Idle,
        }
    }
}

impl SyncState {
    pub fn controls(&self) -> Controls {
        self.status.controls()
    }

    pub fn flashing_at(&self, now: Instant) -> bool {
        self.status.is_running() && self.flash_until.is_some_and(|until| now < until)
    }

    pub fn stale_at(&self, now: Instant) -> bool {
        matches!(self.status, RunPhase::Running(_))
            && self
                .last_scene_at
                .is_some_and(|at| now.saturating_duration_since(at) > STALE_AFTER)
    }
}

pub fn toast_duration(kind: ToastKind) -> Duration {
    match kind {
        ToastKind::Error => Duration::from_secs(6),
        ToastKind::Info | ToastKind::Success => Duration::from_secs(4),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{FormMode, NavDirection, Screen, SyncState, nav_direction};
    use crate::app::sync::run::{RunPhase, STALE_AFTER};
    use crate::screen::flash::FLASH_HOLD;

    #[test]
    fn flashing_expires_after_the_hold_while_paused() {
        let now = Instant::now();
        let mut state = SyncState {
            status: RunPhase::Paused(1),
            flash_until: Some(now + FLASH_HOLD),
            ..SyncState::default()
        };

        assert!(state.flashing_at(now + Duration::from_secs(1)));
        assert!(!state.flashing_at(now + FLASH_HOLD));

        state.status = RunPhase::Idle;
        assert!(!state.flashing_at(now));
    }

    #[test]
    fn stale_follows_the_frame_clock_and_the_run() {
        let now = Instant::now();
        let later = now + STALE_AFTER + Duration::from_secs(1);
        let mut state = SyncState {
            status: RunPhase::Running(1),
            last_scene_at: Some(now),
            ..SyncState::default()
        };

        assert!(!state.stale_at(now));
        assert!(state.stale_at(later));

        state.last_scene_at = Some(later);
        assert!(!state.stale_at(later));

        state.status = RunPhase::Paused(1);
        assert!(!state.stale_at(later));

        state.status = RunPhase::Stopping(1);
        assert!(!state.stale_at(later));
    }

    #[test]
    fn a_run_without_a_frame_is_not_stale_yet() {
        let now = Instant::now();
        let later = now + Duration::from_secs(60);
        let mut state = SyncState {
            status: RunPhase::Starting(1),
            ..SyncState::default()
        };

        assert!(!state.stale_at(later));

        state.status = RunPhase::Running(1);
        assert!(!state.stale_at(later));
    }

    #[test]
    fn section_changes_push_in_tab_order() {
        assert_eq!(
            nav_direction(&Screen::Dashboard, &Screen::Sync),
            NavDirection::Forward
        );
        assert_eq!(
            nav_direction(&Screen::Sync, &Screen::Settings),
            NavDirection::Forward
        );
        assert_eq!(
            nav_direction(&Screen::Settings, &Screen::Dashboard),
            NavDirection::Backward
        );
    }

    #[test]
    fn subpages_open_forward_and_return_backward() {
        assert_eq!(
            nav_direction(&Screen::Dashboard, &Screen::Form(FormMode::Add)),
            NavDirection::Forward
        );
        assert_eq!(
            nav_direction(
                &Screen::Form(FormMode::Edit("id".to_owned())),
                &Screen::Dashboard
            ),
            NavDirection::Backward
        );
        assert_eq!(
            nav_direction(&Screen::Settings, &Screen::Remove("id".to_owned())),
            NavDirection::Forward
        );
    }

    #[test]
    fn leaving_a_subpage_follows_the_tab_order() {
        let form = Screen::Form(FormMode::Add);
        assert_eq!(nav_direction(&form, &Screen::Sync), NavDirection::Forward);
        assert_eq!(
            nav_direction(&form, &Screen::Settings),
            NavDirection::Forward
        );
        assert_eq!(
            nav_direction(&form, &Screen::Dashboard),
            NavDirection::Backward
        );
    }
}
