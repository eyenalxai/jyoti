#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CapturePhase {
    #[default]
    Idle,
    Starting(u64),
    Running(u64),
    Paused(u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureEvent {
    Start(u64),
    Started(u64),
    Pause,
    Resume,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureRejection {
    NotIdle,
    NotStarting,
    NotRunning,
    NotPaused,
    NotActive,
    GenerationMismatch,
}

impl std::fmt::Display for CaptureRejection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptureRejection::NotIdle => formatter.write_str("Screen capture is already active."),
            CaptureRejection::NotStarting => formatter.write_str("Screen capture has not started."),
            CaptureRejection::NotRunning => formatter.write_str("Screen capture is not running."),
            CaptureRejection::NotPaused => formatter.write_str("Screen capture is not paused."),
            CaptureRejection::NotActive => formatter.write_str("Screen capture is not active."),
            CaptureRejection::GenerationMismatch => {
                formatter.write_str("Screen capture generation does not match.")
            }
        }
    }
}

impl std::error::Error for CaptureRejection {}

impl CapturePhase {
    pub fn transition(self, event: CaptureEvent) -> Result<Self, CaptureRejection> {
        match event {
            CaptureEvent::Start(generation) => match self {
                CapturePhase::Idle => Ok(CapturePhase::Starting(generation)),
                _ => Err(CaptureRejection::NotIdle),
            },
            CaptureEvent::Started(generation) => match self {
                CapturePhase::Starting(current) if current == generation => {
                    Ok(CapturePhase::Running(generation))
                }
                CapturePhase::Starting(_) => Err(CaptureRejection::GenerationMismatch),
                _ => Err(CaptureRejection::NotStarting),
            },
            CaptureEvent::Pause => match self {
                CapturePhase::Running(generation) => Ok(CapturePhase::Paused(generation)),
                _ => Err(CaptureRejection::NotRunning),
            },
            CaptureEvent::Resume => match self {
                CapturePhase::Paused(generation) => Ok(CapturePhase::Running(generation)),
                _ => Err(CaptureRejection::NotPaused),
            },
            CaptureEvent::Stop => match self {
                CapturePhase::Starting(_) | CapturePhase::Running(_) | CapturePhase::Paused(_) => {
                    Ok(CapturePhase::Idle)
                }
                CapturePhase::Idle => Err(CaptureRejection::NotActive),
            },
        }
    }

    pub fn generation(self) -> Option<u64> {
        match self {
            CapturePhase::Idle => None,
            CapturePhase::Starting(generation)
            | CapturePhase::Running(generation)
            | CapturePhase::Paused(generation) => Some(generation),
        }
    }

    pub fn is_running(self) -> bool {
        matches!(self, CapturePhase::Running(_))
    }
}

#[cfg(test)]
mod tests {
    use super::{CaptureEvent, CapturePhase, CaptureRejection};
    use CapturePhase::{Idle, Paused, Running, Starting};

    #[test]
    fn every_edge_follows_the_transition_table() {
        let cases: &[(
            CapturePhase,
            CaptureEvent,
            Result<CapturePhase, CaptureRejection>,
        )] = &[
            (Idle, CaptureEvent::Start(9), Ok(Starting(9))),
            (
                Idle,
                CaptureEvent::Started(9),
                Err(CaptureRejection::NotStarting),
            ),
            (Idle, CaptureEvent::Pause, Err(CaptureRejection::NotRunning)),
            (Idle, CaptureEvent::Resume, Err(CaptureRejection::NotPaused)),
            (Idle, CaptureEvent::Stop, Err(CaptureRejection::NotActive)),
            (
                Starting(7),
                CaptureEvent::Start(8),
                Err(CaptureRejection::NotIdle),
            ),
            (Starting(7), CaptureEvent::Started(7), Ok(Running(7))),
            (
                Starting(7),
                CaptureEvent::Started(8),
                Err(CaptureRejection::GenerationMismatch),
            ),
            (
                Starting(7),
                CaptureEvent::Pause,
                Err(CaptureRejection::NotRunning),
            ),
            (
                Starting(7),
                CaptureEvent::Resume,
                Err(CaptureRejection::NotPaused),
            ),
            (Starting(7), CaptureEvent::Stop, Ok(Idle)),
            (
                Running(7),
                CaptureEvent::Start(8),
                Err(CaptureRejection::NotIdle),
            ),
            (
                Running(7),
                CaptureEvent::Started(7),
                Err(CaptureRejection::NotStarting),
            ),
            (Running(7), CaptureEvent::Pause, Ok(Paused(7))),
            (
                Running(7),
                CaptureEvent::Resume,
                Err(CaptureRejection::NotPaused),
            ),
            (Running(7), CaptureEvent::Stop, Ok(Idle)),
            (
                Paused(7),
                CaptureEvent::Start(8),
                Err(CaptureRejection::NotIdle),
            ),
            (
                Paused(7),
                CaptureEvent::Started(7),
                Err(CaptureRejection::NotStarting),
            ),
            (
                Paused(7),
                CaptureEvent::Pause,
                Err(CaptureRejection::NotRunning),
            ),
            (Paused(7), CaptureEvent::Resume, Ok(Running(7))),
            (Paused(7), CaptureEvent::Stop, Ok(Idle)),
        ];

        for (phase, event, expected) in cases {
            assert_eq!(phase.transition(*event), *expected, "{phase:?} + {event:?}");
        }
    }

    #[test]
    fn a_rejected_command_leaves_the_phase_unchanged() {
        let phase = Running(7);

        assert!(phase.transition(CaptureEvent::Start(8)).is_err());
        assert_eq!(phase, Running(7));
        assert!(phase.transition(CaptureEvent::Stop).is_ok());
        assert_eq!(phase, Running(7));
    }

    #[test]
    fn a_generation_is_known_while_the_capture_is_active() {
        assert_eq!(Idle.generation(), None);
        assert_eq!(Starting(7).generation(), Some(7));
        assert_eq!(Running(7).generation(), Some(7));
        assert_eq!(Paused(7).generation(), Some(7));
        assert!(!Idle.is_running());
        assert!(!Starting(7).is_running());
        assert!(Running(7).is_running());
        assert!(!Paused(7).is_running());
    }
}
