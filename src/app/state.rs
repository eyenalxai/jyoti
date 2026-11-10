use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::app::sync::restore::RestorePlan;
use crate::app::sync::run::{Controls, RunEvent, RunPhase, RunRejection};
use crate::bulb::model::Rgb;
use crate::screen::monitor::{MonitorInfo, Scene};

pub const STALE_AFTER: Duration = Duration::from_secs(5);

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
    use std::time::{Duration, Instant};

    use super::{LocalEdits, STALE_AFTER, SyncState};
    use crate::app::sync::run::{RunPhase, StopStage};
    use crate::screen::flash::FLASH_HOLD;

    fn stopping(generation: u64) -> RunPhase {
        RunPhase::Stopping {
            generation,
            stage: StopStage::AwaitingCapture {
                until: Some(Instant::now()),
                seen: false,
            },
        }
    }

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

        state.status = stopping(1);
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
}
