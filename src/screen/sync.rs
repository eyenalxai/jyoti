use crate::bulb::model::{BulbState, ColorMode};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SyncStatus {
    #[default]
    Idle,
    Starting,
    Running,
    Paused,
    Stopping,
    Error(String),
}

impl SyncStatus {
    pub fn label(&self) -> String {
        match self {
            SyncStatus::Idle => "Idle".to_owned(),
            SyncStatus::Starting => "Starting".to_owned(),
            SyncStatus::Running => "Running".to_owned(),
            SyncStatus::Paused => "Paused".to_owned(),
            SyncStatus::Stopping => "Stopping".to_owned(),
            SyncStatus::Error(message) => format!("Error: {message}"),
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(
            self,
            SyncStatus::Starting | SyncStatus::Running | SyncStatus::Paused
        )
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self,
            SyncStatus::Starting | SyncStatus::Running | SyncStatus::Paused | SyncStatus::Error(_)
        )
    }

    pub fn is_busy(&self) -> bool {
        self.is_active() || matches!(self, SyncStatus::Stopping)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestoreControl {
    Color(crate::bulb::model::Rgb),
    Temperature(u16),
}

pub fn restore_control(state: &BulbState) -> Option<RestoreControl> {
    if state.mode == ColorMode::Temperature
        && let Some(temperature) = state.temperature
    {
        return Some(RestoreControl::Temperature(temperature));
    }
    if let Some(color) = state.color {
        return Some(RestoreControl::Color(color));
    }
    state.temperature.map(RestoreControl::Temperature)
}

#[derive(Clone, Debug, Default)]
pub struct RestoreReport {
    pub restored: usize,
    pub not_restored: Vec<String>,
    pub total: usize,
}

impl RestoreReport {
    pub fn message(&self) -> String {
        if self.total == 0 {
            return "Nothing to restore.".to_owned();
        }
        if self.not_restored.is_empty() {
            return format!("Restored {} of {} bulbs.", self.restored, self.total);
        }
        format!(
            "Restored {} of {} bulbs. Not restored: {}.",
            self.restored,
            self.total,
            self.not_restored.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::RestoreReport;

    #[test]
    fn restore_report_names_the_bulbs_that_did_not_return() {
        let report = RestoreReport {
            restored: 2,
            not_restored: vec!["Shelf lamp".to_owned()],
            total: 3,
        };
        assert_eq!(
            report.message(),
            "Restored 2 of 3 bulbs. Not restored: Shelf lamp."
        );
    }

    #[test]
    fn a_clean_restore_counts_every_bulb() {
        let report = RestoreReport {
            restored: 3,
            not_restored: Vec::new(),
            total: 3,
        };
        assert_eq!(report.message(), "Restored 3 of 3 bulbs.");
    }
}
