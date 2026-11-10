use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::screen::monitor::Scene;
use crate::screen::reducer::ReducerState;
use crate::screen::run::{CaptureEvent, CapturePhase, CaptureRejection};
use crate::screen::sync::{Drove, SyncConfig, SyncFrame, SyncRun};

use super::{Capture, Halt};

const TICK: Duration = Duration::from_millis(667);
const IDLE_WAIT: Duration = Duration::from_secs(3_600);
const NO_MONITOR: &str = "No monitor is selected.";

fn immediate(now: Instant) -> Instant {
    now.checked_sub(TICK).unwrap_or(now)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CaptureStop {
    pub generation: u64,
    pub drove: Drove,
}

pub(crate) struct CaptureRun {
    phase: CapturePhase,
    selected: Option<String>,
    run: Option<SyncRun>,
    reducer: ReducerState,
    last_capture: Option<Instant>,
    stopping: Arc<Halt>,
}

impl CaptureRun {
    pub fn new(stopping: Arc<Halt>) -> Self {
        Self {
            phase: CapturePhase::Idle,
            selected: None,
            run: None,
            reducer: ReducerState::default(),
            last_capture: None,
            stopping,
        }
    }

    pub fn phase(&self) -> CapturePhase {
        self.phase
    }

    #[cfg(test)]
    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    pub fn select_monitor(&mut self, monitor: Option<String>, now: Instant) {
        self.selected = monitor;
        self.last_capture = Some(immediate(now));
    }

    pub fn start(
        &mut self,
        generation: u64,
        monitor: String,
        config: SyncConfig,
        now: Instant,
    ) -> Result<(), CaptureRejection> {
        let phase = self.phase.transition(CaptureEvent::Start(generation))?;
        self.phase = phase;
        self.selected = Some(monitor);
        self.run = Some(SyncRun::new(config));
        self.last_capture = Some(now);
        self.stopping.stand_down(generation);
        Ok(())
    }

    pub fn started(&mut self, generation: u64, now: Instant) -> Result<(), CaptureRejection> {
        self.phase = self.phase.transition(CaptureEvent::Started(generation))?;
        self.last_capture = Some(immediate(now));
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), CaptureRejection> {
        self.phase = self.phase.transition(CaptureEvent::Pause)?;
        Ok(())
    }

    pub fn resume(&mut self, now: Instant) -> Result<(), CaptureRejection> {
        self.phase = self.phase.transition(CaptureEvent::Resume)?;
        self.last_capture = Some(immediate(now));
        Ok(())
    }

    pub fn configure(&mut self, config: SyncConfig) {
        if let Some(run) = self.run.as_mut() {
            run.configure(config);
        }
    }

    pub fn stop(&mut self) -> Option<CaptureStop> {
        self.finish_active()
    }

    pub fn shutdown(&mut self) -> Option<CaptureStop> {
        self.finish_active()
    }

    fn finish_active(&mut self) -> Option<CaptureStop> {
        let generation = self.phase.generation()?;
        self.phase = self.phase.transition(CaptureEvent::Stop).ok()?;
        let drove = match self.run.take() {
            Some(run) => run.drove(),
            None => Drove::default(),
        };
        Some(CaptureStop { generation, drove })
    }

    pub fn wait(&self, now: Instant) -> Duration {
        if !self.phase.is_running() {
            return IDLE_WAIT;
        }
        match self.last_capture {
            Some(last) => TICK.saturating_sub(now.saturating_duration_since(last)),
            None => Duration::ZERO,
        }
    }

    pub fn tick(
        &mut self,
        capture: &mut dyn Capture,
        now: Instant,
    ) -> Result<Option<SyncFrame>, String> {
        if !self.phase.is_running() {
            return Ok(None);
        }
        self.last_capture = Some(now);
        if self.stopping.is_halted() {
            return Ok(None);
        }
        let Some(monitor) = self.selected.as_deref() else {
            return Err(NO_MONITOR.to_owned());
        };
        let scene = capture.capture(monitor, &mut self.reducer)?;
        if self.stopping.is_halted() {
            return Ok(None);
        }
        let Some(run) = self.run.as_mut() else {
            return Err("Screen capture has no run.".to_owned());
        };
        Ok(Some(run.apply(scene, now)))
    }

    pub fn capture_once(&mut self, capture: &mut dyn Capture) -> Result<Scene, String> {
        let Some(monitor) = self.selected.as_deref() else {
            return Err(NO_MONITOR.to_owned());
        };
        capture.capture(monitor, &mut self.reducer)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use super::{CaptureRun, Halt, TICK};
    use crate::bulb::model::Rgb;
    use crate::screen::capture::Capture;
    use crate::screen::monitor::{MonitorInfo, Scene};
    use crate::screen::reducer::ReducerState;
    use crate::screen::run::{CapturePhase, CaptureRejection};
    use crate::screen::sync::SyncConfig;

    struct FakeCapture {
        scenes: VecDeque<Scene>,
        captures: usize,
        fail: Option<String>,
    }

    impl FakeCapture {
        fn new(scenes: impl IntoIterator<Item = Scene>) -> Self {
            Self {
                scenes: scenes.into_iter().collect(),
                captures: 0,
                fail: None,
            }
        }
    }

    impl Capture for FakeCapture {
        fn monitors(&mut self) -> Vec<MonitorInfo> {
            Vec::new()
        }

        fn first_monitor(&self) -> Option<String> {
            Some("DP-1".to_owned())
        }

        fn capture(
            &mut self,
            _monitor: &str,
            _reducer: &mut ReducerState,
        ) -> Result<Scene, String> {
            self.captures += 1;
            if let Some(fail) = &self.fail {
                return Err(fail.clone());
            }
            self.scenes
                .pop_front()
                .ok_or_else(|| "The fake capture has no scene.".to_owned())
        }
    }

    fn config() -> SyncConfig {
        SyncConfig {
            gain: 1.0,
            intensity: 0.0,
            targets: Vec::new(),
        }
    }

    fn scene() -> Scene {
        Scene {
            color: Rgb::new(255, 0, 0),
            brightness: 1.0,
        }
    }

    fn running(now: Instant) -> (CaptureRun, FakeCapture, Arc<Halt>) {
        let halt = Arc::new(Halt::new());
        let mut run = CaptureRun::new(Arc::clone(&halt));
        let capture = FakeCapture::new([scene(), scene()]);
        assert!(run.start(7, "DP-1".to_owned(), config(), now).is_ok());
        assert!(run.started(7, now).is_ok());
        assert_eq!(run.phase(), CapturePhase::Running(7));
        (run, capture, halt)
    }

    #[test]
    fn a_fake_capture_drives_start_pause_resume_stop_and_shutdown()
    -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();
        let (mut run, mut capture, _halt) = running(now);
        assert_eq!(run.selected(), Some("DP-1"));

        assert!(run.tick(&mut capture, now)?.is_some());
        assert_eq!(capture.captures, 1);

        run.pause()?;
        assert_eq!(run.phase(), CapturePhase::Paused(7));
        assert!(run.tick(&mut capture, now)?.is_none());
        assert_eq!(capture.captures, 1);

        run.resume(now)?;
        assert_eq!(run.phase(), CapturePhase::Running(7));
        assert!(run.tick(&mut capture, now + TICK)?.is_some());
        assert_eq!(capture.captures, 2);

        let stop = run.stop().ok_or("the run should stop")?;
        assert_eq!(stop.generation, 7);
        assert_eq!(run.phase(), CapturePhase::Idle);
        assert!(run.stop().is_none());
        assert!(run.shutdown().is_none());
        Ok(())
    }

    #[test]
    fn a_new_start_is_accepted_after_a_stop() -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();
        let (mut run, _capture, _halt) = running(now);
        let stop = run.stop().ok_or("the run should stop")?;
        assert_eq!(stop.generation, 7);

        run.start(8, "DP-1".to_owned(), config(), now)?;

        assert_eq!(run.phase(), CapturePhase::Starting(8));
        Ok(())
    }

    #[test]
    fn shutdown_while_running_ends_the_run() -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();
        let (mut run, _capture, _halt) = running(now);

        let stop = run.shutdown().ok_or("the running capture should stop")?;

        assert_eq!(stop.generation, 7);
        assert_eq!(run.phase(), CapturePhase::Idle);
        Ok(())
    }

    #[test]
    fn a_rejected_command_leaves_the_phase_unchanged() -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();
        let mut run = CaptureRun::new(Arc::new(Halt::new()));

        assert_eq!(run.pause(), Err(CaptureRejection::NotRunning));
        assert_eq!(run.phase(), CapturePhase::Idle);
        assert_eq!(run.resume(now), Err(CaptureRejection::NotPaused));
        assert_eq!(run.phase(), CapturePhase::Idle);
        assert!(run.stop().is_none());
        assert_eq!(run.phase(), CapturePhase::Idle);

        run.start(7, "DP-1".to_owned(), config(), now)?;
        assert_eq!(run.pause(), Err(CaptureRejection::NotRunning));
        assert_eq!(run.phase(), CapturePhase::Starting(7));
        Ok(())
    }

    #[test]
    fn a_start_acknowledgement_outside_starting_is_rejected()
    -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();
        let mut run = CaptureRun::new(Arc::new(Halt::new()));
        assert_eq!(run.started(7, now), Err(CaptureRejection::NotStarting));

        run.start(7, "DP-1".to_owned(), config(), now)?;
        run.started(7, now)?;
        assert_eq!(run.started(7, now), Err(CaptureRejection::NotStarting));

        run.pause()?;
        assert_eq!(run.started(7, now), Err(CaptureRejection::NotStarting));

        let _ = run.stop();
        assert_eq!(run.started(7, now), Err(CaptureRejection::NotStarting));
        Ok(())
    }

    #[test]
    fn a_start_acknowledgement_must_match_the_generation() -> Result<(), Box<dyn std::error::Error>>
    {
        let now = Instant::now();
        let mut run = CaptureRun::new(Arc::new(Halt::new()));
        run.start(7, "DP-1".to_owned(), config(), now)?;

        assert_eq!(
            run.started(8, now),
            Err(CaptureRejection::GenerationMismatch)
        );
        assert_eq!(run.phase(), CapturePhase::Starting(7));
        Ok(())
    }

    #[test]
    fn a_halted_tick_does_not_capture() -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();
        let (mut run, mut capture, halt) = running(now);
        halt.halt(7);

        assert!(run.tick(&mut capture, now)?.is_none());
        assert_eq!(capture.captures, 0);
        Ok(())
    }

    #[test]
    fn a_capture_error_is_reported() -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();
        let (mut run, mut capture, _halt) = running(now);
        capture.fail = Some("The monitor is gone.".to_owned());

        assert_eq!(
            run.tick(&mut capture, now).err(),
            Some("The monitor is gone.".to_owned())
        );
        Ok(())
    }

    #[test]
    fn the_wait_is_zero_after_start_and_resume() -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();
        let mut run = CaptureRun::new(Arc::new(Halt::new()));
        assert_eq!(run.wait(now), Duration::from_secs(3_600));

        run.start(7, "DP-1".to_owned(), config(), now)?;
        run.started(7, now)?;
        assert_eq!(run.wait(now), Duration::ZERO);

        run.pause()?;
        assert_eq!(run.wait(now), Duration::from_secs(3_600));

        run.resume(now)?;
        assert_eq!(run.wait(now), Duration::ZERO);
        Ok(())
    }
}
