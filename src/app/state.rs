use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use rune::TextEdit;

use crate::bulb::model::{BulbField, BulbState, Rgb};
use crate::screen::monitor::{MonitorInfo, Scene};
use crate::screen::sync::{RestoreReport, SyncStatus};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Screen {
    Dashboard,
    Sync,
    Settings,
    Form(FormMode),
    Remove(String),
    Color,
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
    pub status: SyncStatus,
    pub monitors: Vec<MonitorInfo>,
    pub capture_error: Option<String>,
    pub selected_monitor: Option<String>,
    pub gain: f32,
    pub colour_intensity: f32,
    pub last_scene: Option<Scene>,
    pub last_scene_at: Option<Instant>,
    pub derived: Option<Rgb>,
    pub brightness: u8,
    pub flashing: bool,
    pub stale: bool,
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
            status: SyncStatus::Idle,
            monitors: Vec::new(),
            capture_error: None,
            selected_monitor: None,
            gain: crate::screen::gain::SYNC_GAIN_DEFAULT,
            colour_intensity: crate::screen::intensity::SYNC_INTENSITY_DEFAULT,
            last_scene: None,
            last_scene_at: None,
            derived: None,
            brightness: 0,
            flashing: false,
            stale: false,
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

pub fn toast_duration(kind: ToastKind) -> Duration {
    match kind {
        ToastKind::Error => Duration::from_secs(6),
        ToastKind::Info | ToastKind::Success => Duration::from_secs(4),
    }
}
