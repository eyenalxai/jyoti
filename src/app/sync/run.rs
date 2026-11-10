use std::time::Instant;

use crate::app::sync::restore::RESTORE_WINDOW;
use crate::screen::run::{CaptureEvent, CapturePhase};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum RunPhase {
    #[default]
    Idle,
    Starting(u64),
    Running(u64),
    Paused(u64),
    Stopping {
        generation: u64,
        stage: StopStage,
    },
    Failed {
        generation: u64,
        message: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopStage {
    AwaitingCapture { until: Option<Instant>, seen: bool },
    Restoring { deadline: Instant },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunEvent {
    Start(u64),
    Stop,
    Stopped(u64),
    Restored(u64),
    Fail { generation: u64, message: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunAdvance {
    Unchanged,
    BeginRestore { deadline: Instant },
    RestoreDue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Controls {
    pub start: bool,
    pub pause: bool,
    pub resume: bool,
    pub stop: bool,
    pub manual: bool,
}

impl Controls {
    const NONE: Self = Self {
        start: false,
        pause: false,
        resume: false,
        stop: false,
        manual: false,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunRejection {
    NotIdle,
    NotRunning,
    NotStopping,
    NotRestoring,
    NotActive,
    GenerationMismatch,
}

impl RunPhase {
    pub fn transition(&self, event: RunEvent) -> Result<RunPhase, RunRejection> {
        match event {
            RunEvent::Start(generation) => match self {
                RunPhase::Idle => CapturePhase::Idle
                    .transition(CaptureEvent::Start(generation))
                    .map(RunPhase::from)
                    .map_err(|_| RunRejection::NotIdle),
                _ => Err(RunRejection::NotIdle),
            },
            RunEvent::Stop => match self {
                RunPhase::Starting(generation)
                | RunPhase::Running(generation)
                | RunPhase::Paused(generation)
                | RunPhase::Failed { generation, .. } => Ok(RunPhase::Stopping {
                    generation: *generation,
                    stage: StopStage::AwaitingCapture {
                        until: None,
                        seen: false,
                    },
                }),
                RunPhase::Idle | RunPhase::Stopping { .. } => Err(RunRejection::NotActive),
            },
            RunEvent::Stopped(generation) => match self {
                RunPhase::Stopping {
                    generation: current,
                    stage,
                } if *current == generation => match stage {
                    StopStage::AwaitingCapture { until, .. } => Ok(RunPhase::Stopping {
                        generation: *current,
                        stage: StopStage::AwaitingCapture {
                            until: *until,
                            seen: true,
                        },
                    }),
                    StopStage::Restoring { .. } => Ok(self.clone()),
                },
                RunPhase::Stopping { .. } => Err(RunRejection::GenerationMismatch),
                _ => Err(RunRejection::NotStopping),
            },
            RunEvent::Restored(generation) => match self {
                RunPhase::Stopping {
                    generation: current,
                    stage,
                } if *current == generation => match stage {
                    StopStage::Restoring { .. } => Ok(RunPhase::Idle),
                    StopStage::AwaitingCapture { .. } => Err(RunRejection::NotRestoring),
                },
                RunPhase::Stopping { .. } => Err(RunRejection::GenerationMismatch),
                _ => Err(RunRejection::NotStopping),
            },
            RunEvent::Fail {
                generation,
                message,
            } => match self {
                RunPhase::Starting(current)
                | RunPhase::Running(current)
                | RunPhase::Paused(current)
                    if *current == generation =>
                {
                    Ok(RunPhase::Failed {
                        generation,
                        message,
                    })
                }
                RunPhase::Starting(_) | RunPhase::Running(_) | RunPhase::Paused(_) => {
                    Err(RunRejection::GenerationMismatch)
                }
                _ => Err(RunRejection::NotRunning),
            },
        }
    }

    pub fn advance(&mut self, now: Instant) -> RunAdvance {
        let Some(stage) = self.stop_stage() else {
            return RunAdvance::Unchanged;
        };
        match stage {
            StopStage::AwaitingCapture { until, seen } => {
                let until = match until {
                    Some(until) => until,
                    None => {
                        let until = now + RESTORE_WINDOW;
                        if let RunPhase::Stopping { stage, .. } = self {
                            *stage = StopStage::AwaitingCapture {
                                until: Some(until),
                                seen,
                            };
                        }
                        if !seen {
                            return RunAdvance::Unchanged;
                        }
                        until
                    }
                };
                if !seen && now < until {
                    return RunAdvance::Unchanged;
                }
                let deadline = now + RESTORE_WINDOW;
                if let RunPhase::Stopping { stage, .. } = self {
                    *stage = StopStage::Restoring { deadline };
                }
                RunAdvance::BeginRestore { deadline }
            }
            StopStage::Restoring { deadline } if now >= deadline => RunAdvance::RestoreDue,
            StopStage::Restoring { .. } => RunAdvance::Unchanged,
        }
    }

    pub fn stop_stage(&self) -> Option<StopStage> {
        match self {
            RunPhase::Stopping { stage, .. } => Some(*stage),
            _ => None,
        }
    }

    /// Stores the phase the capture thread reported. Starting, Running and
    /// Paused belong to the thread; Stopping and Failed stay here. A report
    /// for another generation, or an idle report, is ignored.
    pub fn report(&mut self, phase: CapturePhase) -> bool {
        if !self.is_in_run() {
            return false;
        }
        if phase.generation() != self.generation() {
            return false;
        }
        *self = phase.into();
        true
    }

    pub fn label(&self) -> String {
        match self {
            RunPhase::Idle => "Idle".to_owned(),
            RunPhase::Starting(_) => "Starting".to_owned(),
            RunPhase::Running(_) => "Running".to_owned(),
            RunPhase::Paused(_) => "Paused".to_owned(),
            RunPhase::Stopping { stage, .. } => match stage {
                StopStage::AwaitingCapture { .. } => "Stopping".to_owned(),
                StopStage::Restoring { .. } => "Restoring".to_owned(),
            },
            RunPhase::Failed { message, .. } => format!("Error: {message}"),
        }
    }

    pub fn controls(&self) -> Controls {
        match self {
            RunPhase::Idle => Controls {
                start: true,
                manual: true,
                ..Controls::NONE
            },
            RunPhase::Starting(_) => Controls {
                stop: true,
                ..Controls::NONE
            },
            RunPhase::Running(_) => Controls {
                pause: true,
                stop: true,
                ..Controls::NONE
            },
            RunPhase::Paused(_) => Controls {
                resume: true,
                stop: true,
                ..Controls::NONE
            },
            RunPhase::Stopping { .. } => Controls::NONE,
            RunPhase::Failed { .. } => Controls {
                stop: true,
                ..Controls::NONE
            },
        }
    }

    pub fn generation(&self) -> Option<u64> {
        match self {
            RunPhase::Idle => None,
            RunPhase::Starting(generation)
            | RunPhase::Running(generation)
            | RunPhase::Paused(generation)
            | RunPhase::Stopping { generation, .. }
            | RunPhase::Failed { generation, .. } => Some(*generation),
        }
    }

    pub fn is_in_run(&self) -> bool {
        matches!(
            self,
            RunPhase::Starting(_) | RunPhase::Running(_) | RunPhase::Paused(_)
        )
    }

    pub fn is_stopping(&self) -> bool {
        matches!(self, RunPhase::Stopping { .. })
    }

    pub fn is_active(&self) -> bool {
        self.is_in_run() || matches!(self, RunPhase::Failed { .. })
    }
}

impl From<CapturePhase> for RunPhase {
    fn from(phase: CapturePhase) -> Self {
        match phase {
            CapturePhase::Idle => RunPhase::Idle,
            CapturePhase::Starting(generation) => RunPhase::Starting(generation),
            CapturePhase::Running(generation) => RunPhase::Running(generation),
            CapturePhase::Paused(generation) => RunPhase::Paused(generation),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{RunAdvance, RunEvent, RunPhase, RunRejection, StopStage};
    use crate::app::sync::restore::RESTORE_WINDOW;
    use crate::screen::run::CapturePhase;

    fn failed() -> RunPhase {
        RunPhase::Failed {
            generation: 7,
            message: "capture failed".to_owned(),
        }
    }

    fn stopping(generation: u64) -> RunPhase {
        RunPhase::Stopping {
            generation,
            stage: StopStage::AwaitingCapture {
                until: None,
                seen: false,
            },
        }
    }

    fn waiting(generation: u64, until: Instant) -> RunPhase {
        RunPhase::Stopping {
            generation,
            stage: StopStage::AwaitingCapture {
                until: Some(until),
                seen: false,
            },
        }
    }

    fn seen(generation: u64, until: Option<Instant>) -> RunPhase {
        RunPhase::Stopping {
            generation,
            stage: StopStage::AwaitingCapture { until, seen: true },
        }
    }

    fn restoring(generation: u64, deadline: Instant) -> RunPhase {
        RunPhase::Stopping {
            generation,
            stage: StopStage::Restoring { deadline },
        }
    }

    #[test]
    fn every_edge_follows_the_transition_table() {
        let deadline = Instant::now() + RESTORE_WINDOW;
        let cases: &[(RunPhase, RunEvent, Result<RunPhase, RunRejection>)] = &[
            (
                RunPhase::Idle,
                RunEvent::Start(9),
                Ok(RunPhase::Starting(9)),
            ),
            (RunPhase::Idle, RunEvent::Stop, Err(RunRejection::NotActive)),
            (
                RunPhase::Idle,
                RunEvent::Stopped(7),
                Err(RunRejection::NotStopping),
            ),
            (
                RunPhase::Idle,
                RunEvent::Restored(7),
                Err(RunRejection::NotStopping),
            ),
            (
                RunPhase::Idle,
                RunEvent::Fail {
                    generation: 7,
                    message: "capture failed".to_owned(),
                },
                Err(RunRejection::NotRunning),
            ),
            (
                RunPhase::Starting(7),
                RunEvent::Start(8),
                Err(RunRejection::NotIdle),
            ),
            (RunPhase::Starting(7), RunEvent::Stop, Ok(stopping(7))),
            (
                RunPhase::Starting(7),
                RunEvent::Stopped(7),
                Err(RunRejection::NotStopping),
            ),
            (
                RunPhase::Starting(7),
                RunEvent::Restored(7),
                Err(RunRejection::NotStopping),
            ),
            (
                RunPhase::Starting(7),
                RunEvent::Fail {
                    generation: 7,
                    message: "capture failed".to_owned(),
                },
                Ok(failed()),
            ),
            (
                RunPhase::Running(7),
                RunEvent::Start(8),
                Err(RunRejection::NotIdle),
            ),
            (RunPhase::Running(7), RunEvent::Stop, Ok(stopping(7))),
            (
                RunPhase::Running(7),
                RunEvent::Stopped(7),
                Err(RunRejection::NotStopping),
            ),
            (
                RunPhase::Running(7),
                RunEvent::Restored(7),
                Err(RunRejection::NotStopping),
            ),
            (
                RunPhase::Running(7),
                RunEvent::Fail {
                    generation: 7,
                    message: "capture failed".to_owned(),
                },
                Ok(failed()),
            ),
            (
                RunPhase::Running(7),
                RunEvent::Fail {
                    generation: 8,
                    message: "capture failed".to_owned(),
                },
                Err(RunRejection::GenerationMismatch),
            ),
            (
                RunPhase::Paused(7),
                RunEvent::Start(8),
                Err(RunRejection::NotIdle),
            ),
            (RunPhase::Paused(7), RunEvent::Stop, Ok(stopping(7))),
            (
                RunPhase::Paused(7),
                RunEvent::Stopped(7),
                Err(RunRejection::NotStopping),
            ),
            (
                RunPhase::Paused(7),
                RunEvent::Restored(7),
                Err(RunRejection::NotStopping),
            ),
            (
                RunPhase::Paused(7),
                RunEvent::Fail {
                    generation: 7,
                    message: "capture failed".to_owned(),
                },
                Ok(failed()),
            ),
            (stopping(7), RunEvent::Start(8), Err(RunRejection::NotIdle)),
            (stopping(7), RunEvent::Stop, Err(RunRejection::NotActive)),
            (stopping(7), RunEvent::Stopped(7), Ok(seen(7, None))),
            (
                stopping(7),
                RunEvent::Stopped(8),
                Err(RunRejection::GenerationMismatch),
            ),
            (
                stopping(7),
                RunEvent::Restored(7),
                Err(RunRejection::NotRestoring),
            ),
            (
                stopping(7),
                RunEvent::Fail {
                    generation: 7,
                    message: "capture failed".to_owned(),
                },
                Err(RunRejection::NotRunning),
            ),
            (
                restoring(7, deadline),
                RunEvent::Start(8),
                Err(RunRejection::NotIdle),
            ),
            (
                restoring(7, deadline),
                RunEvent::Stop,
                Err(RunRejection::NotActive),
            ),
            (
                restoring(7, deadline),
                RunEvent::Stopped(7),
                Ok(restoring(7, deadline)),
            ),
            (
                restoring(7, deadline),
                RunEvent::Stopped(8),
                Err(RunRejection::GenerationMismatch),
            ),
            (
                restoring(7, deadline),
                RunEvent::Restored(7),
                Ok(RunPhase::Idle),
            ),
            (
                restoring(7, deadline),
                RunEvent::Restored(8),
                Err(RunRejection::GenerationMismatch),
            ),
            (
                restoring(7, deadline),
                RunEvent::Fail {
                    generation: 7,
                    message: "capture failed".to_owned(),
                },
                Err(RunRejection::NotRunning),
            ),
            (failed(), RunEvent::Start(8), Err(RunRejection::NotIdle)),
            (failed(), RunEvent::Stop, Ok(stopping(7))),
            (
                failed(),
                RunEvent::Stopped(7),
                Err(RunRejection::NotStopping),
            ),
            (
                failed(),
                RunEvent::Restored(7),
                Err(RunRejection::NotStopping),
            ),
            (
                failed(),
                RunEvent::Fail {
                    generation: 7,
                    message: "again".to_owned(),
                },
                Err(RunRejection::NotRunning),
            ),
        ];

        for (phase, event, expected) in cases {
            assert_eq!(
                &phase.transition(event.clone()),
                expected,
                "{} + {event:?}",
                phase.label()
            );
        }
    }

    #[test]
    fn controls_follow_the_phase() {
        let idle = RunPhase::Idle.controls();
        assert!(idle.start);
        assert!(idle.manual);
        assert!(!idle.pause);
        assert!(!idle.resume);
        assert!(!idle.stop);

        let starting = RunPhase::Starting(7).controls();
        assert!(!starting.start);
        assert!(!starting.pause);
        assert!(!starting.resume);
        assert!(starting.stop);
        assert!(!starting.manual);

        let running = RunPhase::Running(7).controls();
        assert!(running.pause);
        assert!(!running.resume);
        assert!(running.stop);
        assert!(!running.manual);

        let paused = RunPhase::Paused(7).controls();
        assert!(!paused.pause);
        assert!(paused.resume);
        assert!(paused.stop);
        assert!(!paused.manual);

        let stopping = stopping(7).controls();
        assert!(!stopping.start);
        assert!(!stopping.pause);
        assert!(!stopping.resume);
        assert!(!stopping.stop);
        assert!(!stopping.manual);

        let failed = failed().controls();
        assert!(!failed.start);
        assert!(!failed.pause);
        assert!(!failed.resume);
        assert!(failed.stop);
        assert!(!failed.manual);
    }

    #[test]
    fn a_stop_before_the_start_acknowledgement_ends_in_a_restore()
    -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();
        let until = now + RESTORE_WINDOW;
        let mut phase = RunPhase::Idle
            .transition(RunEvent::Start(7))
            .map_err(|_| "a start during idle is accepted")?;
        phase = phase
            .transition(RunEvent::Stop)
            .map_err(|_| "a stop during starting is accepted")?;

        assert_eq!(phase, stopping(7));
        assert_eq!(phase.advance(now), RunAdvance::Unchanged);
        assert_eq!(phase, waiting(7, until));
        phase.report(CapturePhase::Running(7));
        assert_eq!(phase, waiting(7, until));
        phase.report(CapturePhase::Paused(7));
        assert_eq!(phase, waiting(7, until));
        phase = phase
            .transition(RunEvent::Stopped(7))
            .map_err(|_| "the capture acknowledgement marks the stage seen")?;
        assert_eq!(phase, seen(7, Some(until)));

        let deadline = now + RESTORE_WINDOW;
        assert_eq!(phase.advance(now), RunAdvance::BeginRestore { deadline });
        assert_eq!(phase.transition(RunEvent::Restored(7)), Ok(RunPhase::Idle));
        Ok(())
    }

    #[test]
    fn a_failed_run_still_restores_on_stop() -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();
        let until = now + RESTORE_WINDOW;
        let mut phase = failed()
            .transition(RunEvent::Stop)
            .map_err(|_| "a stop of a failed run is accepted")?;
        assert_eq!(phase, stopping(7));
        assert_eq!(phase.advance(now), RunAdvance::Unchanged);
        assert_eq!(phase, waiting(7, until));

        phase = phase
            .transition(RunEvent::Stopped(7))
            .map_err(|_| "the capture acknowledgement arrives")?;
        assert_eq!(phase, seen(7, Some(until)));
        let deadline = now + RESTORE_WINDOW;
        assert_eq!(phase.advance(now), RunAdvance::BeginRestore { deadline });
        assert_eq!(phase.transition(RunEvent::Restored(7)), Ok(RunPhase::Idle));
        Ok(())
    }

    #[test]
    fn advance_waits_for_the_capture_acknowledgement_and_the_restore_deadline() {
        let now = Instant::now();
        let until = now + RESTORE_WINDOW;
        let mut phase = waiting(7, until);

        assert_eq!(
            phase.advance(until - Duration::from_millis(1)),
            RunAdvance::Unchanged
        );
        assert_eq!(
            phase.advance(until),
            RunAdvance::BeginRestore {
                deadline: until + RESTORE_WINDOW,
            }
        );

        let deadline = until + RESTORE_WINDOW;
        assert_eq!(
            phase.advance(deadline - Duration::from_millis(1)),
            RunAdvance::Unchanged
        );
        assert_eq!(phase.advance(deadline), RunAdvance::RestoreDue);
        assert_eq!(phase.transition(RunEvent::Restored(7)), Ok(RunPhase::Idle));
    }

    #[test]
    fn the_capture_report_is_stored() {
        let mut phase = RunPhase::Starting(7);

        phase.report(CapturePhase::Running(7));
        assert_eq!(phase, RunPhase::Running(7));

        phase.report(CapturePhase::Paused(7));
        assert_eq!(phase, RunPhase::Paused(7));

        phase.report(CapturePhase::Running(7));
        assert_eq!(phase, RunPhase::Running(7));
    }

    #[test]
    fn a_stale_generation_report_is_ignored() {
        let mut phase = RunPhase::Running(7);

        phase.report(CapturePhase::Paused(6));
        assert_eq!(phase, RunPhase::Running(7));

        phase.report(CapturePhase::Starting(8));
        assert_eq!(phase, RunPhase::Running(7));

        phase.report(CapturePhase::Idle);
        assert_eq!(phase, RunPhase::Running(7));
    }

    #[test]
    fn stopping_and_failed_stay_app_side() {
        let mut phase = stopping(7);
        phase.report(CapturePhase::Running(7));
        assert_eq!(phase, stopping(7));
        phase.report(CapturePhase::Idle);
        assert_eq!(phase, stopping(7));

        let mut phase = failed();
        phase.report(CapturePhase::Running(7));
        assert_eq!(phase, failed());
    }

    #[test]
    fn a_stale_generation_acknowledgement_is_ignored() {
        let now = Instant::now();
        let deadline = now + RESTORE_WINDOW;
        let mut phase = restoring(7, deadline);

        assert_eq!(
            phase.transition(RunEvent::Stopped(7)),
            Ok(restoring(7, deadline))
        );
        assert_eq!(
            phase.transition(RunEvent::Restored(8)),
            Err(RunRejection::GenerationMismatch)
        );
        assert_eq!(
            phase.transition(RunEvent::Stopped(8)),
            Err(RunRejection::GenerationMismatch)
        );
        assert_eq!(
            phase.advance(deadline - Duration::from_millis(1)),
            RunAdvance::Unchanged
        );
        assert_eq!(phase.transition(RunEvent::Restored(7)), Ok(RunPhase::Idle));
    }
}
