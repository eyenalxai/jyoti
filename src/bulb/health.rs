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
pub enum HealthLevel {
    #[default]
    Connecting,
    Online,
    Unreachable,
    Failed,
}

impl HealthLevel {
    pub fn is_down(self) -> bool {
        matches!(self, HealthLevel::Unreachable | HealthLevel::Failed)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HealthState {
    Connecting { message: Option<String> },
    Online { message: Option<String> },
    Unreachable { message: Option<String>, run: bool },
    Failed { message: String },
}

impl Default for HealthState {
    fn default() -> Self {
        Self::Connecting { message: None }
    }
}

impl HealthState {
    pub fn level(&self) -> HealthLevel {
        match self {
            HealthState::Connecting { .. } => HealthLevel::Connecting,
            HealthState::Online { .. } => HealthLevel::Online,
            HealthState::Unreachable { .. } => HealthLevel::Unreachable,
            HealthState::Failed { .. } => HealthLevel::Failed,
        }
    }

    pub fn is_down(&self) -> bool {
        self.level().is_down()
    }

    pub fn message(&self) -> Option<&str> {
        match self {
            HealthState::Connecting { message }
            | HealthState::Online { message }
            | HealthState::Unreachable { message, .. } => message.as_deref(),
            HealthState::Failed { message } => Some(message),
        }
    }

    /// A standing failure is retried by a run; only a failure the run itself
    /// marked keeps the bulb out of its targets.
    pub fn run_unreachable(&self) -> bool {
        matches!(self, HealthState::Unreachable { run: true, .. })
    }

    fn set_message(&mut self, message: String) {
        match self {
            HealthState::Connecting { message: current }
            | HealthState::Online { message: current }
            | HealthState::Unreachable {
                message: current, ..
            } => *current = Some(message),
            HealthState::Failed { .. } => {}
        }
    }

    fn clear_run_mark(&mut self) -> bool {
        match self {
            HealthState::Unreachable { run, .. } if *run => {
                *run = false;
                true
            }
            _ => false,
        }
    }
}

/// One bulb's typed health plus the consecutive in-run failure count. The
/// record owns both, so callers never carry a second health flag.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Health {
    state: HealthState,
    run_failures: u8,
}

impl Health {
    pub fn level(&self) -> HealthLevel {
        self.state.level()
    }

    pub fn is_down(&self) -> bool {
        self.state.is_down()
    }

    pub fn message(&self) -> Option<&str> {
        self.state.message()
    }

    pub fn run_unreachable(&self) -> bool {
        self.state.run_unreachable()
    }

    pub fn advance(&mut self, event: HealthEvent, in_run: bool) -> bool {
        if matches!(self.state, HealthState::Failed { .. }) {
            return false;
        }
        let before = self.state.clone();
        match event {
            HealthEvent::State | HealthEvent::Sent | HealthEvent::Reachable(true) => {
                self.state = HealthState::Online { message: None };
                self.run_failures = 0;
            }
            HealthEvent::Reachable(false) => {
                let message = self.state.message().map(str::to_owned);
                self.state = HealthState::Unreachable {
                    message,
                    run: in_run,
                };
                self.run_failures = 0;
            }
            HealthEvent::Error(message) => {
                if in_run {
                    self.run_failures = self.run_failures.saturating_add(1);
                    if self.run_failures >= RUN_FAILURE_LIMIT {
                        self.state = HealthState::Unreachable {
                            message: Some(message),
                            run: true,
                        };
                    } else {
                        self.state.set_message(message);
                    }
                } else {
                    self.state = HealthState::Unreachable {
                        message: Some(message),
                        run: false,
                    };
                    self.run_failures = 0;
                }
            }
            HealthEvent::Exited => {
                let message = self
                    .state
                    .message()
                    .map_or_else(|| LOST_MESSAGE.to_owned(), str::to_owned);
                self.state = HealthState::Failed { message };
                self.run_failures = 0;
            }
        }
        self.state != before
    }

    pub fn begin_run(&mut self) {
        self.run_failures = 0;
        self.state.clear_run_mark();
    }

    pub fn end_run(&mut self) -> bool {
        self.run_failures = 0;
        let mark = self.state.clear_run_mark();
        let message = match &mut self.state {
            HealthState::Connecting { message } | HealthState::Online { message } => {
                message.take().is_some()
            }
            HealthState::Unreachable { .. } | HealthState::Failed { .. } => false,
        };
        mark || message
    }
}

#[cfg(test)]
mod tests {
    use super::{Health, HealthEvent, HealthLevel, HealthState};

    fn error(health: &mut Health, message: &str, in_run: bool) -> bool {
        health.advance(HealthEvent::Error(message.to_owned()), in_run)
    }

    #[test]
    fn one_failure_outside_a_run_is_unreachable() {
        let mut health = Health::default();
        assert_eq!(health.level(), HealthLevel::Connecting);

        assert!(error(&mut health, "no answer", false));
        assert_eq!(health.level(), HealthLevel::Unreachable);
        assert!(health.is_down());
        assert!(!health.run_unreachable());
        assert_eq!(health.message(), Some("no answer"));

        assert!(!error(&mut health, "no answer", false));
    }

    #[test]
    fn the_first_two_in_run_failures_stay_visible_without_leaving_the_run() {
        let mut health = Health::default();

        assert!(error(&mut health, "one", true));
        assert_eq!(health.level(), HealthLevel::Connecting);
        assert!(!health.is_down());
        assert_eq!(health.message(), Some("one"));

        assert!(error(&mut health, "two", true));
        assert_eq!(health.level(), HealthLevel::Connecting);
        assert_eq!(health.message(), Some("two"));
    }

    #[test]
    fn three_failures_inside_a_run_are_run_unreachable() {
        let mut health = Health::default();

        assert!(error(&mut health, "one", true));
        assert!(error(&mut health, "two", true));
        assert!(!health.is_down());
        assert!(!health.run_unreachable());

        assert!(error(&mut health, "three", true));
        assert_eq!(health.level(), HealthLevel::Unreachable);
        assert!(health.run_unreachable());
        assert_eq!(health.message(), Some("three"));
    }

    #[test]
    fn a_run_end_preserves_the_standing_failure_and_clears_only_the_mark() {
        let mut health = Health::default();
        for message in ["one", "two", "three"] {
            error(&mut health, message, true);
        }
        assert!(health.run_unreachable());

        assert!(health.end_run());
        assert_eq!(health.level(), HealthLevel::Unreachable);
        assert!(!health.run_unreachable());
        assert_eq!(health.message(), Some("three"));

        assert!(!health.end_run());
    }

    #[test]
    fn a_run_end_clears_a_transient_in_run_message() {
        let mut health = Health::default();
        error(&mut health, "one", true);
        assert_eq!(health.message(), Some("one"));

        assert!(health.end_run());
        assert_eq!(health.level(), HealthLevel::Connecting);
        assert_eq!(health.message(), None);
    }

    #[test]
    fn a_failed_probe_inside_a_run_is_run_unreachable() {
        let mut health = Health::default();
        health.advance(HealthEvent::Reachable(false), true);
        assert!(health.run_unreachable());

        health.end_run();
        assert!(health.is_down());
        assert!(!health.run_unreachable());
    }

    #[test]
    fn a_failed_probe_after_a_run_stays_visible() {
        let mut health = Health::default();
        health.advance(HealthEvent::Reachable(false), false);
        health.end_run();

        assert!(health.is_down());
        assert!(!health.run_unreachable());
    }

    #[test]
    fn a_run_retries_a_standing_failure_but_keeps_it_visible() {
        let mut health = Health::default();
        health.advance(HealthEvent::Error("no answer".to_owned()), false);
        assert!(health.is_down());
        assert!(!health.run_unreachable());

        health.begin_run();
        for message in ["one", "two", "three"] {
            error(&mut health, message, true);
        }
        assert!(health.run_unreachable());

        health.end_run();
        assert!(health.is_down());
        assert!(!health.run_unreachable());
        assert!(health.message().is_some());
    }

    #[test]
    fn a_run_that_recovers_a_standing_failure_clears_it() {
        let mut health = Health::default();
        health.advance(HealthEvent::Error("no answer".to_owned()), false);

        health.begin_run();
        health.advance(HealthEvent::State, true);
        health.end_run();

        assert_eq!(health.level(), HealthLevel::Online);
        assert!(!health.is_down());
        assert_eq!(health.message(), None);
    }

    #[test]
    fn a_recovered_bulb_leaves_the_run_count() {
        let mut health = Health::default();
        health.begin_run();
        for message in ["one", "two", "three"] {
            error(&mut health, message, true);
        }
        assert!(health.is_down());

        health.advance(HealthEvent::Sent, true);
        health.end_run();

        assert!(!health.is_down());
        assert!(!health.run_unreachable());
    }

    #[test]
    fn a_lost_worker_is_failed_once_however_often_it_is_reported() {
        let mut health = Health::default();
        assert!(health.advance(HealthEvent::Exited, false));
        assert_eq!(health.level(), HealthLevel::Failed);
        assert!(health.is_down());
        assert!(health.message().is_some());

        assert!(!health.advance(HealthEvent::Exited, false));
        health.end_run();
        assert_eq!(health.level(), HealthLevel::Failed);
        assert!(health.message().is_some());
    }

    #[test]
    fn a_failed_health_is_terminal_until_a_new_worker() {
        let mut health = Health::default();
        health.advance(HealthEvent::Exited, false);

        assert!(!health.advance(HealthEvent::State, false));
        assert_eq!(health.level(), HealthLevel::Failed);

        health = Health::default();
        assert_eq!(health.level(), HealthLevel::Connecting);
    }

    #[test]
    fn the_typed_state_keeps_the_message_with_the_level() {
        let mut state = HealthState::Online { message: None };
        state.set_message("slow".to_owned());
        assert_eq!(state.level(), HealthLevel::Online);
        assert_eq!(state.message(), Some("slow"));
    }
}
