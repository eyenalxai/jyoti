#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum RunPhase {
    #[default]
    Idle,
    Starting(u64),
    Running(u64),
    Paused(u64),
    Stopping(u64),
    Failed {
        generation: u64,
        message: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunEvent {
    Start(u64),
    Started(u64),
    Frame,
    Pause,
    Resume,
    Stop,
    Stopped(u64),
    Restored,
    Fail { generation: u64, message: String },
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
    NotStarting,
    NotRunning,
    NotPaused,
    NotStopping,
    NotActive,
    GenerationMismatch,
}

impl RunPhase {
    pub fn transition(&self, event: RunEvent) -> Result<RunPhase, RunRejection> {
        match event {
            RunEvent::Start(generation) => match self {
                RunPhase::Idle => Ok(RunPhase::Starting(generation)),
                _ => Err(RunRejection::NotIdle),
            },
            RunEvent::Started(generation) => match self {
                RunPhase::Starting(current) if *current == generation => {
                    Ok(RunPhase::Running(generation))
                }
                RunPhase::Starting(_) => Err(RunRejection::GenerationMismatch),
                _ => Err(RunRejection::NotStarting),
            },
            RunEvent::Frame => match self {
                RunPhase::Running(_) => Ok(self.clone()),
                _ => Err(RunRejection::NotRunning),
            },
            RunEvent::Pause => match self {
                RunPhase::Running(generation) => Ok(RunPhase::Paused(*generation)),
                _ => Err(RunRejection::NotRunning),
            },
            RunEvent::Resume => match self {
                RunPhase::Paused(generation) => Ok(RunPhase::Running(*generation)),
                _ => Err(RunRejection::NotPaused),
            },
            RunEvent::Stop => match self {
                RunPhase::Starting(generation)
                | RunPhase::Running(generation)
                | RunPhase::Paused(generation) => Ok(RunPhase::Stopping(*generation)),
                RunPhase::Failed { generation, .. } => Ok(RunPhase::Stopping(*generation)),
                RunPhase::Idle | RunPhase::Stopping(_) => Err(RunRejection::NotActive),
            },
            RunEvent::Stopped(generation) => match self {
                RunPhase::Stopping(current) if *current == generation => Ok(self.clone()),
                RunPhase::Stopping(_) => Err(RunRejection::GenerationMismatch),
                _ => Err(RunRejection::NotStopping),
            },
            RunEvent::Restored => match self {
                RunPhase::Stopping(_) => Ok(RunPhase::Idle),
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

    pub fn label(&self) -> String {
        match self {
            RunPhase::Idle => "Idle".to_owned(),
            RunPhase::Starting(_) => "Starting".to_owned(),
            RunPhase::Running(_) => "Running".to_owned(),
            RunPhase::Paused(_) => "Paused".to_owned(),
            RunPhase::Stopping(_) => "Stopping".to_owned(),
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
            RunPhase::Stopping(_) => Controls::NONE,
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
            | RunPhase::Stopping(generation)
            | RunPhase::Failed { generation, .. } => Some(*generation),
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(
            self,
            RunPhase::Starting(_) | RunPhase::Running(_) | RunPhase::Paused(_)
        )
    }

    pub fn is_active(&self) -> bool {
        self.is_running() || matches!(self, RunPhase::Failed { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::{RunEvent, RunPhase, RunRejection};

    fn failed() -> RunPhase {
        RunPhase::Failed {
            generation: 7,
            message: "capture failed".to_owned(),
        }
    }

    #[test]
    fn every_edge_follows_the_transition_table() {
        let cases: &[(RunPhase, RunEvent, Result<RunPhase, RunRejection>)] = &[
            (
                RunPhase::Idle,
                RunEvent::Start(9),
                Ok(RunPhase::Starting(9)),
            ),
            (
                RunPhase::Idle,
                RunEvent::Started(7),
                Err(RunRejection::NotStarting),
            ),
            (
                RunPhase::Idle,
                RunEvent::Frame,
                Err(RunRejection::NotRunning),
            ),
            (
                RunPhase::Idle,
                RunEvent::Pause,
                Err(RunRejection::NotRunning),
            ),
            (
                RunPhase::Idle,
                RunEvent::Resume,
                Err(RunRejection::NotPaused),
            ),
            (RunPhase::Idle, RunEvent::Stop, Err(RunRejection::NotActive)),
            (
                RunPhase::Idle,
                RunEvent::Stopped(7),
                Err(RunRejection::NotStopping),
            ),
            (
                RunPhase::Idle,
                RunEvent::Restored,
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
            (
                RunPhase::Starting(7),
                RunEvent::Started(7),
                Ok(RunPhase::Running(7)),
            ),
            (
                RunPhase::Starting(7),
                RunEvent::Started(8),
                Err(RunRejection::GenerationMismatch),
            ),
            (
                RunPhase::Starting(7),
                RunEvent::Frame,
                Err(RunRejection::NotRunning),
            ),
            (
                RunPhase::Starting(7),
                RunEvent::Pause,
                Err(RunRejection::NotRunning),
            ),
            (
                RunPhase::Starting(7),
                RunEvent::Resume,
                Err(RunRejection::NotPaused),
            ),
            (
                RunPhase::Starting(7),
                RunEvent::Stop,
                Ok(RunPhase::Stopping(7)),
            ),
            (
                RunPhase::Starting(7),
                RunEvent::Stopped(7),
                Err(RunRejection::NotStopping),
            ),
            (
                RunPhase::Starting(7),
                RunEvent::Restored,
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
            (
                RunPhase::Running(7),
                RunEvent::Started(7),
                Err(RunRejection::NotStarting),
            ),
            (
                RunPhase::Running(7),
                RunEvent::Frame,
                Ok(RunPhase::Running(7)),
            ),
            (
                RunPhase::Running(7),
                RunEvent::Pause,
                Ok(RunPhase::Paused(7)),
            ),
            (
                RunPhase::Running(7),
                RunEvent::Resume,
                Err(RunRejection::NotPaused),
            ),
            (
                RunPhase::Running(7),
                RunEvent::Stop,
                Ok(RunPhase::Stopping(7)),
            ),
            (
                RunPhase::Running(7),
                RunEvent::Stopped(7),
                Err(RunRejection::NotStopping),
            ),
            (
                RunPhase::Running(7),
                RunEvent::Restored,
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
                RunPhase::Paused(7),
                RunEvent::Start(8),
                Err(RunRejection::NotIdle),
            ),
            (
                RunPhase::Paused(7),
                RunEvent::Started(7),
                Err(RunRejection::NotStarting),
            ),
            (
                RunPhase::Paused(7),
                RunEvent::Frame,
                Err(RunRejection::NotRunning),
            ),
            (
                RunPhase::Paused(7),
                RunEvent::Pause,
                Err(RunRejection::NotRunning),
            ),
            (
                RunPhase::Paused(7),
                RunEvent::Resume,
                Ok(RunPhase::Running(7)),
            ),
            (
                RunPhase::Paused(7),
                RunEvent::Stop,
                Ok(RunPhase::Stopping(7)),
            ),
            (
                RunPhase::Paused(7),
                RunEvent::Stopped(7),
                Err(RunRejection::NotStopping),
            ),
            (
                RunPhase::Paused(7),
                RunEvent::Restored,
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
            (
                RunPhase::Stopping(7),
                RunEvent::Start(8),
                Err(RunRejection::NotIdle),
            ),
            (
                RunPhase::Stopping(7),
                RunEvent::Started(7),
                Err(RunRejection::NotStarting),
            ),
            (
                RunPhase::Stopping(7),
                RunEvent::Frame,
                Err(RunRejection::NotRunning),
            ),
            (
                RunPhase::Stopping(7),
                RunEvent::Pause,
                Err(RunRejection::NotRunning),
            ),
            (
                RunPhase::Stopping(7),
                RunEvent::Resume,
                Err(RunRejection::NotPaused),
            ),
            (
                RunPhase::Stopping(7),
                RunEvent::Stop,
                Err(RunRejection::NotActive),
            ),
            (
                RunPhase::Stopping(7),
                RunEvent::Stopped(7),
                Ok(RunPhase::Stopping(7)),
            ),
            (
                RunPhase::Stopping(7),
                RunEvent::Stopped(8),
                Err(RunRejection::GenerationMismatch),
            ),
            (
                RunPhase::Stopping(7),
                RunEvent::Restored,
                Ok(RunPhase::Idle),
            ),
            (
                RunPhase::Stopping(7),
                RunEvent::Fail {
                    generation: 7,
                    message: "capture failed".to_owned(),
                },
                Err(RunRejection::NotRunning),
            ),
            (failed(), RunEvent::Start(8), Err(RunRejection::NotIdle)),
            (
                failed(),
                RunEvent::Started(8),
                Err(RunRejection::NotStarting),
            ),
            (failed(), RunEvent::Frame, Err(RunRejection::NotRunning)),
            (failed(), RunEvent::Pause, Err(RunRejection::NotRunning)),
            (failed(), RunEvent::Resume, Err(RunRejection::NotPaused)),
            (failed(), RunEvent::Stop, Ok(RunPhase::Stopping(7))),
            (
                failed(),
                RunEvent::Stopped(7),
                Err(RunRejection::NotStopping),
            ),
            (failed(), RunEvent::Restored, Err(RunRejection::NotStopping)),
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

        let stopping = RunPhase::Stopping(7).controls();
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
    fn a_stop_before_the_start_acknowledgement_ends_in_a_restore() {
        assert_eq!(
            RunPhase::Idle.transition(RunEvent::Start(7)),
            Ok(RunPhase::Starting(7))
        );
        assert_eq!(
            RunPhase::Starting(7).transition(RunEvent::Stop),
            Ok(RunPhase::Stopping(7))
        );
        assert_eq!(
            RunPhase::Stopping(7).transition(RunEvent::Started(7)),
            Err(RunRejection::NotStarting)
        );
        assert_eq!(
            RunPhase::Stopping(7).transition(RunEvent::Frame),
            Err(RunRejection::NotRunning)
        );
        assert_eq!(
            RunPhase::Stopping(7).transition(RunEvent::Stopped(7)),
            Ok(RunPhase::Stopping(7))
        );
        assert_eq!(
            RunPhase::Stopping(7).transition(RunEvent::Restored),
            Ok(RunPhase::Idle)
        );
    }
}
