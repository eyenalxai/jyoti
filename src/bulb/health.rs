/// A run publishes at 1.5 Hz, so a single missed reply is normal. A bulb only
/// leaves the run after this many consecutive failures; outside a run one
/// failure is already a failure.
const RUN_FAILURE_LIMIT: u8 = 3;

const LOST_MESSAGE: &str = "The bulb's connection stopped. Refresh the bulb list to reconnect.";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HealthEvent {
    State,
    Sent,
    Error(String),
    Reachable(bool),
    Exited,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HealthStatus {
    #[default]
    Connecting,
    Online,
    Unreachable,
    Failed,
}

impl HealthStatus {
    pub fn is_down(self) -> bool {
        matches!(self, HealthStatus::Unreachable | HealthStatus::Failed)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum HealthState {
    #[default]
    Connecting,
    Online,
    Unreachable {
        message: Option<String>,
        run: bool,
    },
    Failed {
        message: String,
    },
}

impl HealthState {
    pub fn status(&self) -> HealthStatus {
        match self {
            HealthState::Connecting => HealthStatus::Connecting,
            HealthState::Online => HealthStatus::Online,
            HealthState::Unreachable { .. } => HealthStatus::Unreachable,
            HealthState::Failed { .. } => HealthStatus::Failed,
        }
    }

    pub fn is_down(&self) -> bool {
        self.status().is_down()
    }

    pub fn message(&self) -> Option<&str> {
        match self {
            HealthState::Unreachable { message, .. } => message.as_deref(),
            HealthState::Failed { message } => Some(message),
            HealthState::Connecting | HealthState::Online => None,
        }
    }

    /// A standing failure is retried by a run; only a failure the run itself
    /// marked keeps the bulb out of its targets.
    pub fn run_unreachable(&self) -> bool {
        matches!(self, HealthState::Unreachable { run: true, .. })
    }

    pub fn clear_run_mark(&mut self) -> bool {
        match self {
            HealthState::Unreachable { run, .. } if *run => {
                *run = false;
                true
            }
            _ => false,
        }
    }
}

/// One health transition. `run_failures` counts consecutive failures inside a
/// run; the caller owns it so the typed state never carries a duplicate flag.
pub fn advance_health(
    state: &mut HealthState,
    run_failures: &mut u8,
    event: HealthEvent,
    in_run: bool,
) -> bool {
    if matches!(state, HealthState::Failed { .. }) {
        return false;
    }
    let before = state.clone();
    match event {
        HealthEvent::State | HealthEvent::Sent | HealthEvent::Reachable(true) => {
            *state = HealthState::Online;
            *run_failures = 0;
        }
        HealthEvent::Reachable(false) => {
            let message = state.message().map(str::to_owned);
            *state = HealthState::Unreachable {
                message,
                run: in_run,
            };
            *run_failures = 0;
        }
        HealthEvent::Error(message) => {
            if in_run {
                *run_failures = run_failures.saturating_add(1);
                if *run_failures >= RUN_FAILURE_LIMIT {
                    *state = HealthState::Unreachable {
                        message: Some(message),
                        run: true,
                    };
                } else if let HealthState::Unreachable {
                    message: current, ..
                } = state
                {
                    *current = Some(message);
                }
            } else {
                *state = HealthState::Unreachable {
                    message: Some(message),
                    run: false,
                };
                *run_failures = 0;
            }
        }
        HealthEvent::Exited => {
            let message = state
                .message()
                .map_or_else(|| LOST_MESSAGE.to_owned(), str::to_owned);
            *state = HealthState::Failed { message };
        }
    }
    *state != before
}

pub fn end_run_health(state: &mut HealthState) -> bool {
    state.clear_run_mark()
}

#[cfg(test)]
mod tests {
    use super::{HealthEvent, HealthState, HealthStatus, advance_health, end_run_health};

    fn error(state: &mut HealthState, failures: &mut u8, message: &str, in_run: bool) -> bool {
        advance_health(
            state,
            failures,
            HealthEvent::Error(message.to_owned()),
            in_run,
        )
    }

    #[test]
    fn one_failure_outside_a_run_is_unreachable() {
        let mut state = HealthState::default();
        let mut failures = 0;

        assert_eq!(state.status(), HealthStatus::Connecting);
        assert!(error(&mut state, &mut failures, "no answer", false));

        assert_eq!(state.status(), HealthStatus::Unreachable);
        assert!(state.is_down());
        assert!(!state.run_unreachable());
        assert_eq!(state.message(), Some("no answer"));

        assert!(!error(&mut state, &mut failures, "no answer", false));
    }

    #[test]
    fn three_failures_inside_a_run_are_run_unreachable() {
        let mut state = HealthState::Online;
        let mut failures = 0;

        assert!(!error(&mut state, &mut failures, "one", true));
        assert!(!error(&mut state, &mut failures, "two", true));
        assert!(!state.is_down());
        assert!(!state.run_unreachable());

        assert!(error(&mut state, &mut failures, "three", true));
        assert_eq!(state.status(), HealthStatus::Unreachable);
        assert!(state.run_unreachable());
        assert_eq!(state.message(), Some("three"));
    }

    #[test]
    fn a_run_end_preserves_the_standing_failure_and_clears_only_the_mark() {
        let mut state = HealthState::Online;
        let mut failures = 0;
        for message in ["one", "two", "three"] {
            error(&mut state, &mut failures, message, true);
        }
        assert!(state.run_unreachable());

        assert!(end_run_health(&mut state));
        assert_eq!(state.status(), HealthStatus::Unreachable);
        assert!(!state.run_unreachable());
        assert_eq!(state.message(), Some("three"));

        assert!(!end_run_health(&mut state));
    }

    #[test]
    fn a_failed_probe_inside_a_run_is_run_unreachable() {
        let mut state = HealthState::Online;
        let mut failures = 0;

        assert!(advance_health(
            &mut state,
            &mut failures,
            HealthEvent::Reachable(false),
            true
        ));
        assert!(state.run_unreachable());

        end_run_health(&mut state);
        assert!(state.is_down());
        assert!(!state.run_unreachable());
    }

    #[test]
    fn a_run_retries_a_standing_failure_but_keeps_it_visible() {
        let mut state = HealthState::default();
        let mut failures = 0;
        error(&mut state, &mut failures, "no answer", false);
        assert!(state.is_down());
        assert!(!state.run_unreachable());

        for message in ["one", "two", "three"] {
            error(&mut state, &mut failures, message, true);
        }
        assert!(state.run_unreachable());

        end_run_health(&mut state);
        assert!(state.is_down());
        assert!(!state.run_unreachable());
        assert_eq!(state.message(), Some("three"));
    }

    #[test]
    fn a_run_that_recovers_a_standing_failure_clears_it() {
        let mut state = HealthState::default();
        let mut failures = 0;
        error(&mut state, &mut failures, "no answer", false);

        assert!(advance_health(
            &mut state,
            &mut failures,
            HealthEvent::State,
            true
        ));

        assert_eq!(state.status(), HealthStatus::Online);
        assert!(!state.is_down());
        assert_eq!(state.message(), None);
    }

    #[test]
    fn three_failures_outside_a_run_stay_a_standing_failure() {
        let mut state = HealthState::default();
        let mut failures = 0;

        for message in ["one", "two", "three"] {
            error(&mut state, &mut failures, message, false);
        }

        assert!(state.is_down());
        assert!(!state.run_unreachable());
        assert_eq!(state.message(), Some("three"));
    }

    #[test]
    fn a_recovered_bulb_starts_the_run_count_again() {
        let mut state = HealthState::Online;
        let mut failures = 0;
        for message in ["one", "two", "three"] {
            error(&mut state, &mut failures, message, true);
        }
        assert!(state.run_unreachable());

        advance_health(&mut state, &mut failures, HealthEvent::Sent, true);
        assert!(!state.is_down());

        assert!(!error(&mut state, &mut failures, "one", true));
        assert!(!error(&mut state, &mut failures, "two", true));
        assert!(!state.is_down());

        assert!(error(&mut state, &mut failures, "three", true));
        assert!(state.run_unreachable());
    }

    #[test]
    fn a_worker_exit_is_failed_once() {
        let mut state = HealthState::default();
        let mut failures = 0;

        assert!(advance_health(
            &mut state,
            &mut failures,
            HealthEvent::Exited,
            false
        ));
        assert_eq!(state.status(), HealthStatus::Failed);
        assert!(state.message().is_some());

        assert!(!advance_health(
            &mut state,
            &mut failures,
            HealthEvent::Exited,
            false
        ));
        assert_eq!(state.status(), HealthStatus::Failed);
    }

    #[test]
    fn a_failed_worker_stays_failed() {
        let mut state = HealthState::default();
        let mut failures = 0;
        advance_health(&mut state, &mut failures, HealthEvent::Exited, false);

        assert!(!advance_health(
            &mut state,
            &mut failures,
            HealthEvent::State,
            false
        ));
        assert_eq!(state.status(), HealthStatus::Failed);
        assert!(!end_run_health(&mut state));
        assert_eq!(state.status(), HealthStatus::Failed);
    }
}
