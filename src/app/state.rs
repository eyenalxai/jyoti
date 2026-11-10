use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::app::sync::restore::RestorePlan;
use crate::app::sync::run::{Controls, RunEvent, RunPhase, RunRejection};
use crate::bulb::model::Rgb;
use crate::screen::monitor::{MonitorInfo, Scene};
use crate::screen::run::CapturePhase;

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

    pub fn report(&mut self, phase: CapturePhase) -> bool {
        self.status.report(phase)
    }

    pub fn controls(&self) -> Controls {
        self.status.controls()
    }

    pub fn flashing_at(&self, now: Instant) -> bool {
        self.status.is_in_run() && self.flash_until.is_some_and(|until| now < until)
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

    use super::{STALE_AFTER, SyncState};
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
