use std::collections::HashMap;
use std::time::{Duration, Instant};

use rune::TextEdit;

use crate::app::sync::restore::{RestorePlan, StopPhase};
use crate::app::sync::run::{Controls, RunEvent, RunPhase, RunRejection};
use crate::bulb::model::{BulbField, BulbState, Rgb};
use crate::screen::color::rgb_to_hsb;
use crate::screen::monitor::{MonitorInfo, Scene};

pub const STALE_AFTER: Duration = Duration::from_secs(5);

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Overlay {
    #[default]
    None,
    BulbPicker,
    BulbMenu,
    MonitorPicker,
}

impl Overlay {
    pub fn toggle(self, overlay: Self) -> Self {
        if self == overlay { Self::None } else { overlay }
    }

    pub fn close(self, overlay: Self) -> Self {
        if self == overlay { Self::None } else { self }
    }
}

pub enum ColorEditor {
    Closed(Box<TextEdit>),
    Open(Box<ColorSession>),
}

pub struct ColorSession {
    pub targets: Vec<String>,
    pub hue: f32,
    pub saturation: f32,
    pub value: f32,
    pub hex: TextEdit,
    pub preview: Rgb,
    pub error: Option<String>,
    pre_open: HashMap<String, BulbState>,
}

impl ColorSession {
    fn new(targets: Vec<String>, color: Rgb, pre_open: HashMap<String, BulbState>) -> Self {
        let hsb = rgb_to_hsb(color);
        Self {
            targets,
            hue: hsb.hue,
            saturation: hsb.saturation,
            value: hsb.value,
            hex: TextEdit::new(color.hex()),
            preview: color,
            error: None,
            pre_open,
        }
    }

    pub fn cancel(self) -> HashMap<String, BulbState> {
        self.pre_open
    }
}

impl ColorEditor {
    pub fn open(targets: Vec<String>, color: Rgb, pre_open: HashMap<String, BulbState>) -> Self {
        Self::Open(Box::new(ColorSession::new(targets, color, pre_open)))
    }

    pub fn is_open(&self) -> bool {
        matches!(self, Self::Open(_))
    }

    pub fn session(&self) -> Option<&ColorSession> {
        match self {
            Self::Closed(_) => None,
            Self::Open(session) => Some(session),
        }
    }

    pub fn session_mut(&mut self) -> Option<&mut ColorSession> {
        match self {
            Self::Closed(_) => None,
            Self::Open(session) => Some(session),
        }
    }

    // The text input binds to a `&mut TextEdit`; a closed editor keeps an empty
    // buffer for that binding, but no targets or pre-open values.
    pub fn hex_mut(&mut self) -> &mut TextEdit {
        match self {
            Self::Closed(hex) => hex.as_mut(),
            Self::Open(session) => &mut session.hex,
        }
    }

    pub fn take(&mut self) -> Option<ColorSession> {
        match std::mem::replace(self, Self::Closed(Box::new(TextEdit::new("")))) {
            Self::Open(session) => Some(*session),
            Self::Closed(_) => None,
        }
    }

    pub fn close(&mut self) {
        *self = Self::Closed(Box::new(TextEdit::new("")));
    }
}

impl Default for ColorEditor {
    fn default() -> Self {
        Self::Closed(Box::new(TextEdit::new("")))
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

/// A worker reads a bulb's state over the network. Its reply is only current if
/// no control moved the same bulb after the read began.
#[derive(Default)]
pub struct LocalEdits {
    at: HashMap<String, Instant>,
}

impl LocalEdits {
    pub fn touch(&mut self, id: &str) {
        self.mark_at(id, Instant::now());
    }

    pub fn mark_at(&mut self, id: &str, at: Instant) {
        self.at.insert(id.to_owned(), at);
    }

    pub fn accepts(&self, id: &str, observed_at: Instant) -> bool {
        self.at
            .get(id)
            .is_none_or(|edited_at| observed_at >= *edited_at)
    }

    pub fn remove(&mut self, id: &str) {
        self.at.remove(id);
    }

    pub fn clear(&mut self) {
        self.at.clear();
    }
}

pub struct SyncState {
    pub status: RunPhase,
    pub next_generation: u64,
    pub monitors: Vec<MonitorInfo>,
    pub capture_error: Option<String>,
    pub capture_failed: bool,
    pub selected_monitor: Option<String>,
    pub gain: f32,
    pub colour_intensity: f32,
    pub last_scene: Option<Scene>,
    pub last_scene_at: Option<Instant>,
    pub derived: Option<Rgb>,
    pub brightness: u8,
    pub flash_until: Option<Instant>,
    pub restore: RestorePlan,
    pub last_probe: Instant,
    pub stop: StopPhase,
    pub stop_seen: bool,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            status: RunPhase::Idle,
            next_generation: 1,
            monitors: Vec::new(),
            capture_error: None,
            capture_failed: false,
            selected_monitor: None,
            gain: crate::screen::gain::SYNC_GAIN_DEFAULT,
            colour_intensity: crate::screen::intensity::SYNC_INTENSITY_DEFAULT,
            last_scene: None,
            last_scene_at: None,
            derived: None,
            brightness: 0,
            flash_until: None,
            restore: RestorePlan::default(),
            last_probe: Instant::now(),
            stop: StopPhase::Idle,
            stop_seen: false,
        }
    }
}

impl SyncState {
    pub fn apply(&mut self, event: RunEvent) -> Result<(), RunRejection> {
        self.status = self.status.transition(event)?;
        Ok(())
    }

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
    use std::collections::HashMap;
    use std::time::{Duration, Instant};

    use super::{
        ColorEditor, FormMode, LocalEdits, NavDirection, Overlay, STALE_AFTER, Screen, SyncState,
        nav_direction,
    };
    use crate::app::sync::run::RunPhase;
    use crate::bulb::model::{BulbState, ColorMode, Rgb};
    use crate::screen::flash::FLASH_HOLD;

    #[test]
    fn a_state_read_that_started_before_an_edit_is_ignored() {
        let mut edits = LocalEdits::default();
        let read_started = Instant::now();
        let edited = read_started + Duration::from_millis(20);
        edits.mark_at("desk", edited);

        assert!(!edits.accepts("desk", read_started));
        assert!(edits.accepts("desk", edited));
        assert!(edits.accepts("desk", edited + Duration::from_millis(1)));
        assert!(edits.accepts("shelf", read_started));
    }

    #[test]
    fn forgetting_a_bulb_drops_its_edit_stamp() {
        let mut edits = LocalEdits::default();
        let read_started = Instant::now();
        edits.mark_at("desk", read_started + Duration::from_secs(1));

        edits.remove("desk");

        assert!(edits.accepts("desk", read_started));
    }

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

    fn state(color: Rgb) -> BulbState {
        BulbState {
            power: Some(true),
            brightness: Some(40),
            color: Some(color),
            mode: ColorMode::Color,
            ..BulbState::default()
        }
    }

    fn pre_open() -> HashMap<String, BulbState> {
        HashMap::from([
            ("desk".to_owned(), state(Rgb::new(10, 20, 30))),
            ("shelf".to_owned(), state(Rgb::new(40, 50, 60))),
        ])
    }

    #[test]
    fn only_one_overlay_is_open() {
        assert_eq!(
            Overlay::None.toggle(Overlay::BulbPicker),
            Overlay::BulbPicker
        );
        assert_eq!(
            Overlay::BulbPicker.toggle(Overlay::BulbMenu),
            Overlay::BulbMenu
        );
        assert_eq!(Overlay::BulbMenu.toggle(Overlay::BulbMenu), Overlay::None);
        assert_eq!(
            Overlay::BulbMenu.close(Overlay::BulbPicker),
            Overlay::BulbMenu
        );
        assert_eq!(Overlay::BulbMenu.close(Overlay::BulbMenu), Overlay::None);
    }

    #[test]
    fn a_closed_editor_has_no_session() {
        let mut editor = ColorEditor::default();

        assert!(!editor.is_open());
        assert!(editor.session().is_none());
        assert!(editor.session_mut().is_none());
        assert!(editor.take().is_none());
        assert!(!editor.is_open());
    }

    #[test]
    fn opening_captures_targets_and_pre_open_values() -> Result<(), Box<dyn std::error::Error>> {
        let before = pre_open();
        let mut editor = ColorEditor::open(
            vec!["desk".to_owned(), "shelf".to_owned()],
            Rgb::new(255, 128, 0),
            before.clone(),
        );

        let session = editor.session().ok_or("the editor should be open")?;
        assert_eq!(session.targets, vec!["desk".to_owned(), "shelf".to_owned()]);
        assert_eq!(session.preview, Rgb::new(255, 128, 0));
        assert_eq!(session.hex.text, "#FF8000");

        let cancelled = editor.take().ok_or("the editor should be open")?.cancel();
        assert_eq!(cancelled, before);
        assert!(!editor.is_open());
        Ok(())
    }

    #[test]
    fn closing_keeps_the_preview_and_drops_the_pre_open_values() {
        let mut editor =
            ColorEditor::open(vec!["desk".to_owned()], Rgb::new(255, 128, 0), pre_open());
        let preview = editor.session().map(|session| session.preview);

        editor.close();

        assert_eq!(preview, Some(Rgb::new(255, 128, 0)));
        assert!(!editor.is_open());
        assert!(editor.take().is_none());
    }
}
