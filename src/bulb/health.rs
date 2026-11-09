use std::collections::{HashMap, HashSet};

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

#[derive(Clone, Debug, Default)]
struct Record {
    answered: bool,
    unreachable: bool,
    run_unreachable: bool,
    run_failures: u8,
    exited: bool,
    message: Option<String>,
}

impl Record {
    fn apply(&mut self, event: HealthEvent, in_run: bool) -> bool {
        let was = (self.status(), self.message.clone());
        match event {
            HealthEvent::State | HealthEvent::Sent | HealthEvent::Reachable(true) => {
                self.answered = true;
                self.unreachable = false;
                self.run_unreachable = false;
                self.run_failures = 0;
                self.message = None;
            }
            HealthEvent::Reachable(false) => {
                if in_run {
                    self.run_unreachable = true;
                } else {
                    self.unreachable = true;
                }
            }
            HealthEvent::Error(message) => {
                self.message = Some(message);
                if in_run {
                    self.run_failures = self.run_failures.saturating_add(1);
                    if self.run_failures >= RUN_FAILURE_LIMIT {
                        self.run_unreachable = true;
                    }
                } else {
                    self.unreachable = true;
                }
            }
            HealthEvent::Exited => {
                self.exited = true;
                self.message.get_or_insert_with(|| LOST_MESSAGE.to_owned());
            }
        }
        self.status() != was.0 || self.message != was.1
    }

    fn status(&self) -> HealthStatus {
        if self.exited {
            HealthStatus::Failed
        } else if self.run_unreachable || self.unreachable {
            HealthStatus::Unreachable
        } else if self.answered {
            HealthStatus::Online
        } else {
            HealthStatus::Connecting
        }
    }

    fn begin_run(&mut self) {
        self.run_unreachable = false;
        self.run_failures = 0;
    }

    fn end_run(&mut self) {
        self.run_unreachable = false;
        self.run_failures = 0;
        if !self.unreachable && !self.exited {
            self.message = None;
        }
    }
}

#[derive(Default)]
pub struct HealthSet {
    records: HashMap<String, Record>,
    in_run: bool,
}

impl HealthSet {
    pub fn apply(&mut self, id: &str, event: HealthEvent) -> bool {
        let in_run = self.in_run;
        self.records
            .entry(id.to_owned())
            .or_default()
            .apply(event, in_run)
    }

    pub fn begin_run(&mut self) {
        self.in_run = true;
        for record in self.records.values_mut() {
            record.begin_run();
        }
    }

    pub fn end_run(&mut self) {
        self.in_run = false;
        for record in self.records.values_mut() {
            record.end_run();
        }
    }

    pub fn remove(&mut self, id: &str) {
        self.records.remove(id);
    }

    pub fn retain(&mut self, keep: &HashSet<String>) {
        self.records.retain(|id, _| keep.contains(id));
    }

    pub fn clear(&mut self) {
        self.records.clear();
        self.in_run = false;
    }

    pub fn status(&self, id: &str) -> HealthStatus {
        self.records
            .get(id)
            .map_or(HealthStatus::Connecting, Record::status)
    }

    pub fn message(&self, id: &str) -> Option<&str> {
        self.records.get(id)?.message.as_deref()
    }

    pub fn unreachable(&self, id: &str) -> bool {
        self.status(id).is_down()
    }

    /// A standing failure is retried by a run; only a failure the run itself
    /// marked keeps the bulb out of its targets.
    pub fn run_unreachable(&self, id: &str) -> bool {
        self.records
            .get(id)
            .is_some_and(|record| record.run_unreachable)
    }

    pub fn unreachable_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .records
            .iter()
            .filter(|(_, record)| record.status().is_down())
            .map(|(id, _)| id.clone())
            .collect();
        ids.sort();
        ids
    }
}

#[cfg(test)]
mod tests {
    use super::{HealthEvent, HealthSet, HealthStatus};

    fn error(message: &str) -> HealthEvent {
        HealthEvent::Error(message.to_owned())
    }

    #[test]
    fn a_failed_call_counts_once_until_the_bulb_answers() {
        let mut health = HealthSet::default();
        assert_eq!(health.status("desk"), HealthStatus::Connecting);
        assert!(health.unreachable_ids().is_empty());

        assert!(health.apply("desk", error("The bulb did not answer.")));
        assert_eq!(health.status("desk"), HealthStatus::Unreachable);
        assert_eq!(health.unreachable_ids(), vec!["desk".to_owned()]);

        assert!(!health.apply("desk", error("The bulb did not answer.")));
        assert_eq!(health.unreachable_ids().len(), 1);

        assert!(health.apply("desk", HealthEvent::State));
        assert_eq!(health.status("desk"), HealthStatus::Online);
        assert!(health.unreachable_ids().is_empty());
        assert_eq!(health.message("desk"), None);
    }

    #[test]
    fn a_run_marks_a_bulb_after_three_failures_and_clears_it_at_run_end() {
        let mut health = HealthSet::default();
        health.begin_run();

        health.apply("desk", error("one"));
        health.apply("desk", error("two"));
        assert!(!health.unreachable("desk"));
        assert!(health.unreachable_ids().is_empty());

        assert!(health.apply("desk", error("three")));
        assert!(health.unreachable("desk"));
        assert_eq!(health.unreachable_ids().len(), 1);

        health.end_run();
        assert!(!health.unreachable("desk"));
        assert!(health.unreachable_ids().is_empty());
        assert_eq!(health.status("desk"), HealthStatus::Connecting);
        assert_eq!(health.message("desk"), None);
    }

    #[test]
    fn a_run_retries_a_standing_failure_but_keeps_it_visible() {
        let mut health = HealthSet::default();
        health.apply("desk", error("no answer"));

        health.begin_run();

        assert!(health.unreachable("desk"));
        assert!(!health.run_unreachable("desk"));

        health.apply("desk", error("one"));
        health.apply("desk", error("two"));
        health.apply("desk", error("three"));
        assert!(health.run_unreachable("desk"));

        health.end_run();
        assert!(health.unreachable("desk"));
        assert!(health.message("desk").is_some());
    }

    #[test]
    fn a_run_that_recovers_a_standing_failure_clears_it() {
        let mut health = HealthSet::default();
        health.apply("desk", error("no answer"));

        health.begin_run();
        health.apply("desk", HealthEvent::State);
        health.end_run();

        assert_eq!(health.status("desk"), HealthStatus::Online);
        assert!(health.unreachable_ids().is_empty());
        assert_eq!(health.message("desk"), None);
    }

    #[test]
    fn a_recovered_bulb_leaves_the_run_count() {
        let mut health = HealthSet::default();
        health.begin_run();
        for message in ["one", "two", "three"] {
            health.apply("desk", error(message));
        }
        assert_eq!(health.unreachable_ids().len(), 1);

        health.apply("desk", HealthEvent::Sent);

        assert!(!health.unreachable("desk"));
        assert!(health.unreachable_ids().is_empty());
    }

    #[test]
    fn a_lost_worker_is_one_failure_however_often_it_is_reported() {
        let mut health = HealthSet::default();
        assert!(health.apply("desk", HealthEvent::Exited));
        assert_eq!(health.status("desk"), HealthStatus::Failed);
        assert_eq!(health.unreachable_ids().len(), 1);
        assert!(health.message("desk").is_some());

        assert!(!health.apply("desk", HealthEvent::Exited));
        assert_eq!(health.unreachable_ids().len(), 1);

        health.end_run();
        assert_eq!(health.unreachable_ids().len(), 1);
        assert!(health.message("desk").is_some());
    }

    #[test]
    fn a_stopped_worker_can_start_again_with_a_clean_record() {
        let mut health = HealthSet::default();
        health.apply("desk", HealthEvent::Exited);
        health.remove("desk");

        assert_eq!(health.status("desk"), HealthStatus::Connecting);
        assert!(health.unreachable_ids().is_empty());
    }

    #[test]
    fn a_removed_bulb_leaves_the_count() {
        let mut health = HealthSet::default();
        health.apply("desk", error("gone"));
        health.apply("shelf", error("gone"));

        health.retain(&std::collections::HashSet::from(["desk".to_owned()]));

        assert_eq!(health.unreachable_ids(), vec!["desk".to_owned()]);
    }
}
