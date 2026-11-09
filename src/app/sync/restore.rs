use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::{Duration, Instant};

use crate::bulb::model::{Bulb, BulbState, ColorMode, Rgb};
use crate::screen::sync::Drove;

pub const RESTORE_WINDOW: Duration = Duration::from_millis(2_000);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StopPhase {
    #[default]
    Idle,
    Waiting {
        until: Instant,
    },
    Restoring {
        deadline: Instant,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestoreControl {
    Color(Rgb),
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
    pub not_restored: Vec<String>,
    pub total: usize,
}

impl RestoreReport {
    pub fn restored(&self) -> usize {
        self.total.saturating_sub(self.not_restored.len())
    }

    pub fn message(&self) -> String {
        if self.total == 0 {
            return "Nothing to restore.".to_owned();
        }
        if self.not_restored.is_empty() {
            return format!("Restored {} of {} bulbs.", self.restored(), self.total);
        }
        format!(
            "Restored {} of {} bulbs. Not restored: {}.",
            self.restored(),
            self.total,
            self.not_restored.join(", ")
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestoreControls {
    pub color: Option<RestoreControl>,
    pub brightness: Option<u8>,
    pub incomplete: bool,
}

impl RestoreControls {
    fn is_empty(&self) -> bool {
        self.color.is_none() && self.brightness.is_none()
    }

    pub fn color_and_temperature(&self) -> (Option<Rgb>, Option<u16>) {
        match self.color {
            Some(RestoreControl::Color(value)) => (Some(value), None),
            Some(RestoreControl::Temperature(value)) => (None, Some(value)),
            None => (None, None),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct DrivenControls {
    color: bool,
    brightness: bool,
}

#[derive(Clone, Debug)]
struct ExpectedBulb {
    name: String,
    snapshot: Option<BulbState>,
}

#[derive(Clone, Debug, Default)]
pub struct RestorePlan {
    expected: BTreeMap<String, ExpectedBulb>,
    driven: BTreeMap<String, DrivenControls>,
    waiting: BTreeSet<String>,
    acknowledged: BTreeSet<String>,
    failed: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RestoreRequest {
    Restore {
        id: String,
        controls: RestoreControls,
    },
    MissingValue {
        id: String,
    },
}

impl RestorePlan {
    pub fn begin(&mut self, bulbs: &[Bulb], states: &HashMap<String, BulbState>) {
        self.expected.clear();
        self.driven.clear();
        self.waiting.clear();
        self.acknowledged.clear();
        self.failed.clear();
        for bulb in bulbs {
            self.expected.insert(
                bulb.id.clone(),
                ExpectedBulb {
                    name: bulb.name.clone(),
                    snapshot: states.get(&bulb.id).cloned(),
                },
            );
        }
    }

    pub fn observe(&mut self, id: &str, name: &str, state: &BulbState) -> bool {
        match self.expected.get_mut(id) {
            Some(expected) => {
                if expected.snapshot.is_some() {
                    return false;
                }
                expected.snapshot = Some(state.clone());
                true
            }
            None => {
                self.expected.insert(
                    id.to_owned(),
                    ExpectedBulb {
                        name: name.to_owned(),
                        snapshot: Some(state.clone()),
                    },
                );
                true
            }
        }
    }

    pub fn covers(&self, id: &str) -> bool {
        self.expected
            .get(id)
            .is_some_and(|expected| expected.snapshot.is_some())
    }

    pub fn record(&mut self, drove: &Drove) {
        for id in &drove.color {
            self.record_one(
                id,
                DrivenControls {
                    color: true,
                    brightness: false,
                },
            );
        }
        for id in &drove.brightness {
            self.record_one(
                id,
                DrivenControls {
                    color: false,
                    brightness: true,
                },
            );
        }
    }

    fn record_one(&mut self, id: &str, written: DrivenControls) {
        if !self.covers(id) {
            return;
        }
        let driven = self.driven.entry(id.to_owned()).or_default();
        driven.color |= written.color;
        driven.brightness |= written.brightness;
    }

    pub fn requests(&self) -> Vec<RestoreRequest> {
        self.driven
            .iter()
            .filter(|(id, _)| !self.requested(id))
            .filter_map(|(id, driven)| {
                let controls = self.controls(id, *driven)?;
                if controls.is_empty() {
                    Some(RestoreRequest::MissingValue { id: id.clone() })
                } else {
                    Some(RestoreRequest::Restore {
                        id: id.clone(),
                        controls,
                    })
                }
            })
            .collect()
    }

    pub fn mark_waiting(&mut self, id: &str) {
        self.waiting.insert(id.to_owned());
    }

    pub fn mark_failed(&mut self, id: &str) {
        self.failed.insert(id.to_owned());
    }

    pub fn acknowledge(&mut self, id: &str, ok: bool) {
        self.waiting.remove(id);
        if ok {
            self.acknowledged.insert(id.to_owned());
        } else {
            self.failed.insert(id.to_owned());
        }
    }

    pub fn is_open(&self) -> bool {
        !self.waiting.is_empty()
    }

    pub fn finish(&mut self) -> RestoreReport {
        let waiting: Vec<String> = self.waiting.iter().cloned().collect();
        for id in waiting {
            self.failed.insert(id);
        }
        self.waiting.clear();
        self.report()
    }

    pub fn report(&self) -> RestoreReport {
        let mut total = 0;
        let mut not_restored = Vec::new();
        for (id, expected) in &self.expected {
            if expected.snapshot.is_none() {
                total += 1;
                not_restored.push(expected.name.clone());
                continue;
            }
            let Some(driven) = self.driven.get(id) else {
                continue;
            };
            total += 1;
            let incomplete = self
                .controls(id, *driven)
                .is_none_or(|controls| controls.incomplete);
            if incomplete || !self.acknowledged.contains(id) {
                not_restored.push(expected.name.clone());
            }
        }
        not_restored.sort();
        RestoreReport {
            not_restored,
            total,
        }
    }

    fn requested(&self, id: &str) -> bool {
        self.waiting.contains(id) || self.acknowledged.contains(id) || self.failed.contains(id)
    }

    fn controls(&self, id: &str, driven: DrivenControls) -> Option<RestoreControls> {
        let snapshot = self.expected.get(id)?.snapshot.as_ref()?;
        let mut incomplete = false;
        let color = if driven.color {
            match restore_control(snapshot) {
                Some(value) => Some(value),
                None => {
                    incomplete = true;
                    None
                }
            }
        } else {
            None
        };
        let brightness = if driven.brightness {
            match snapshot.brightness {
                Some(value) => Some(value),
                None => {
                    incomplete = true;
                    None
                }
            }
        } else {
            None
        };
        Some(RestoreControls {
            color,
            brightness,
            incomplete,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{RestoreControl, RestoreControls, RestorePlan, RestoreReport, RestoreRequest};
    use crate::bulb::model::{Bulb, BulbState, ColorMode, Rgb};
    use crate::screen::sync::Drove;

    fn bulb(id: &str, name: &str) -> Bulb {
        Bulb {
            id: id.to_owned(),
            name: name.to_owned(),
            address: "192.168.1.42".to_owned(),
            model: None,
        }
    }

    fn colour_state(color: Rgb, brightness: u8) -> BulbState {
        BulbState {
            power: Some(true),
            brightness: Some(brightness),
            color: Some(color),
            mode: ColorMode::Color,
            ..BulbState::default()
        }
    }

    fn temperature_state(temperature: u16, brightness: u8) -> BulbState {
        BulbState {
            power: Some(true),
            brightness: Some(brightness),
            temperature: Some(temperature),
            mode: ColorMode::Temperature,
            ..BulbState::default()
        }
    }

    fn controls(color: Option<RestoreControl>, brightness: Option<u8>) -> RestoreControls {
        RestoreControls {
            color,
            brightness,
            incomplete: false,
        }
    }

    fn drove(color: &[&str], brightness: &[&str]) -> Drove {
        Drove {
            color: color.iter().map(|id| (*id).to_owned()).collect(),
            brightness: brightness.iter().map(|id| (*id).to_owned()).collect(),
        }
    }

    #[test]
    fn a_configured_bulb_becomes_a_restore_target_when_its_state_is_known() {
        let desk = bulb("desk", "Desk lamp");
        let shelf = bulb("shelf", "Shelf lamp");
        let states = HashMap::from([(desk.id.clone(), colour_state(Rgb::new(255, 0, 0), 40))]);
        let mut plan = RestorePlan::default();

        plan.begin(&[desk, shelf], &states);

        assert!(plan.covers("desk"));
        assert!(!plan.covers("shelf"));
        assert!(plan.observe(
            "shelf",
            "Shelf lamp",
            &colour_state(Rgb::new(0, 255, 0), 70)
        ));
        assert!(plan.covers("shelf"));
    }

    #[test]
    fn a_mid_run_bulb_keeps_the_state_it_joined_with() {
        let mut plan = RestorePlan::default();
        plan.observe("bulb", "Desk lamp", &colour_state(Rgb::new(255, 0, 0), 40));
        plan.observe("bulb", "Desk lamp", &colour_state(Rgb::new(0, 0, 255), 90));
        plan.record(&drove(&["bulb"], &[]));

        assert_eq!(
            plan.requests(),
            vec![RestoreRequest::Restore {
                id: "bulb".to_owned(),
                controls: controls(Some(RestoreControl::Color(Rgb::new(255, 0, 0))), None),
            }]
        );
    }

    #[test]
    fn a_run_that_never_published_sends_no_restore() {
        let desk = bulb("desk", "Desk lamp");
        let states = HashMap::from([(desk.id.clone(), colour_state(Rgb::new(255, 0, 0), 40))]);
        let mut plan = RestorePlan::default();

        plan.begin(&[desk], &states);

        assert!(plan.requests().is_empty());
        assert!(!plan.is_open());
        let report = plan.finish();
        assert_eq!(report.total, 0);
        assert_eq!(report.restored(), 0);
        assert!(report.not_restored.is_empty());
        assert_eq!(report.message(), "Nothing to restore.");
    }

    #[test]
    fn only_the_controls_the_run_wrote_are_restored() {
        let desk = bulb("desk", "Desk lamp");
        let states = HashMap::from([(desk.id.clone(), colour_state(Rgb::new(255, 128, 0), 40))]);
        let mut plan = RestorePlan::default();
        plan.begin(&[desk], &states);

        plan.record(&drove(&["desk"], &[]));

        assert_eq!(
            plan.requests(),
            vec![RestoreRequest::Restore {
                id: "desk".to_owned(),
                controls: controls(Some(RestoreControl::Color(Rgb::new(255, 128, 0))), None),
            }]
        );
    }

    #[test]
    fn a_brightness_write_restores_only_brightness() {
        let desk = bulb("desk", "Desk lamp");
        let states = HashMap::from([(desk.id.clone(), colour_state(Rgb::new(255, 128, 0), 40))]);
        let mut plan = RestorePlan::default();
        plan.begin(&[desk], &states);

        plan.record(&drove(&[], &["desk"]));

        assert_eq!(
            plan.requests(),
            vec![RestoreRequest::Restore {
                id: "desk".to_owned(),
                controls: controls(None, Some(40)),
            }]
        );
    }

    #[test]
    fn a_temperature_snapshot_restores_the_temperature() {
        let desk = bulb("desk", "Desk lamp");
        let states = HashMap::from([(desk.id.clone(), temperature_state(3_000, 40))]);
        let mut plan = RestorePlan::default();
        plan.begin(&[desk], &states);

        plan.record(&drove(&["desk"], &[]));

        assert_eq!(
            plan.requests(),
            vec![RestoreRequest::Restore {
                id: "desk".to_owned(),
                controls: controls(Some(RestoreControl::Temperature(3_000)), None),
            }]
        );
    }

    #[test]
    fn a_partial_snapshot_restores_what_it_has_and_names_the_bulb() {
        let desk = bulb("desk", "Desk lamp");
        let partial = BulbState {
            power: Some(true),
            color: Some(Rgb::new(255, 0, 0)),
            mode: ColorMode::Color,
            ..BulbState::default()
        };
        let states = HashMap::from([(desk.id.clone(), partial)]);
        let mut plan = RestorePlan::default();
        plan.begin(&[desk], &states);
        plan.record(&drove(&["desk"], &["desk"]));

        assert_eq!(
            plan.requests(),
            vec![RestoreRequest::Restore {
                id: "desk".to_owned(),
                controls: RestoreControls {
                    color: Some(RestoreControl::Color(Rgb::new(255, 0, 0))),
                    brightness: None,
                    incomplete: true,
                },
            }]
        );
        plan.mark_waiting("desk");
        plan.acknowledge("desk", true);

        let report = plan.finish();
        assert_eq!(report.total, 1);
        assert_eq!(report.restored(), 0);
        assert_eq!(report.not_restored, vec!["Desk lamp".to_owned()]);
    }

    #[test]
    fn a_missing_restore_value_is_named_as_not_restored() {
        let desk = bulb("desk", "Desk lamp");
        let states = HashMap::from([(desk.id.clone(), BulbState::default())]);
        let mut plan = RestorePlan::default();
        plan.begin(&[desk], &states);
        plan.record(&drove(&["desk"], &[]));

        assert_eq!(
            plan.requests(),
            vec![RestoreRequest::MissingValue {
                id: "desk".to_owned()
            }]
        );
        plan.mark_failed("desk");

        let report = plan.finish();
        assert_eq!(report.total, 1);
        assert_eq!(report.restored(), 0);
        assert_eq!(report.not_restored, vec!["Desk lamp".to_owned()]);
    }

    #[test]
    fn the_plan_waits_for_every_acknowledgement() {
        let desk = bulb("desk", "Desk lamp");
        let shelf = bulb("shelf", "Shelf lamp");
        let states = HashMap::from([
            (desk.id.clone(), colour_state(Rgb::new(255, 0, 0), 40)),
            (shelf.id.clone(), colour_state(Rgb::new(0, 255, 0), 70)),
        ]);
        let mut plan = RestorePlan::default();
        plan.begin(&[desk, shelf], &states);
        plan.record(&drove(&["desk", "shelf"], &["desk", "shelf"]));

        let requests = plan.requests();
        assert_eq!(requests.len(), 2);
        for request in &requests {
            match request {
                RestoreRequest::Restore { id, .. } | RestoreRequest::MissingValue { id } => {
                    plan.mark_waiting(id);
                }
            }
        }
        assert!(plan.is_open());

        plan.acknowledge("desk", true);
        assert!(plan.is_open());

        plan.acknowledge("shelf", true);
        assert!(!plan.is_open());

        let report = plan.finish();
        assert_eq!(report.total, 2);
        assert_eq!(report.restored(), 2);
        assert!(report.not_restored.is_empty());
        assert_eq!(report.message(), "Restored 2 of 2 bulbs.");
    }

    #[test]
    fn the_report_names_the_bulbs_still_pending_at_the_deadline() {
        let desk = bulb("desk", "Desk lamp");
        let shelf = bulb("shelf", "Shelf lamp");
        let states = HashMap::from([
            (desk.id.clone(), colour_state(Rgb::new(255, 0, 0), 40)),
            (shelf.id.clone(), colour_state(Rgb::new(0, 255, 0), 70)),
        ]);
        let mut plan = RestorePlan::default();
        plan.begin(&[desk, shelf], &states);
        plan.record(&drove(&["desk", "shelf"], &["desk", "shelf"]));
        plan.mark_waiting("desk");
        plan.mark_waiting("shelf");
        plan.acknowledge("desk", true);

        let report = plan.finish();
        assert!(!plan.is_open());
        assert_eq!(report.total, 2);
        assert_eq!(report.restored(), 1);
        assert_eq!(report.not_restored, vec!["Shelf lamp".to_owned()]);
        assert_eq!(
            report.message(),
            "Restored 1 of 2 bulbs. Not restored: Shelf lamp."
        );
    }

    #[test]
    fn a_failed_acknowledgement_is_not_restored() {
        let desk = bulb("desk", "Desk lamp");
        let states = HashMap::from([(desk.id.clone(), colour_state(Rgb::new(255, 0, 0), 40))]);
        let mut plan = RestorePlan::default();
        plan.begin(&[desk], &states);
        plan.record(&drove(&["desk"], &[]));
        plan.mark_waiting("desk");

        plan.acknowledge("desk", false);

        assert!(!plan.is_open());
        let report = plan.finish();
        assert_eq!(report.restored(), 0);
        assert_eq!(report.not_restored, vec!["Desk lamp".to_owned()]);
    }

    #[test]
    fn a_configured_bulb_without_a_snapshot_is_named() {
        let desk = bulb("desk", "Desk lamp");
        let shelf = bulb("shelf", "Shelf lamp");
        let states = HashMap::from([(desk.id.clone(), colour_state(Rgb::new(255, 0, 0), 40))]);
        let mut plan = RestorePlan::default();
        plan.begin(&[desk, shelf], &states);
        plan.record(&drove(&["desk"], &[]));
        plan.mark_waiting("desk");
        plan.acknowledge("desk", true);

        let report = plan.finish();
        assert_eq!(report.total, 2);
        assert_eq!(report.restored(), 1);
        assert_eq!(report.not_restored, vec!["Shelf lamp".to_owned()]);
    }

    #[test]
    fn an_unwritten_snapshot_is_not_counted_as_restored() {
        let desk = bulb("desk", "Desk lamp");
        let states = HashMap::from([(desk.id.clone(), colour_state(Rgb::new(255, 0, 0), 40))]);
        let mut plan = RestorePlan::default();
        plan.begin(&[desk], &states);

        let report = plan.finish();
        assert_eq!(report.total, 0);
        assert!(report.not_restored.is_empty());
    }

    #[test]
    fn a_clean_restore_without_bulbs_reports_nothing_to_restore() {
        let mut plan = RestorePlan::default();
        plan.begin(&[], &HashMap::new());

        let report = plan.finish();
        assert_eq!(report.total, 0);
        assert_eq!(report.message(), "Nothing to restore.");
    }

    #[test]
    fn restore_report_names_the_bulbs_that_did_not_return() {
        let report = RestoreReport {
            not_restored: vec!["Shelf lamp".to_owned()],
            total: 3,
        };
        assert_eq!(report.restored(), 2);
        assert_eq!(
            report.message(),
            "Restored 2 of 3 bulbs. Not restored: Shelf lamp."
        );
    }

    #[test]
    fn a_clean_restore_counts_every_bulb() {
        let report = RestoreReport {
            not_restored: Vec::new(),
            total: 3,
        };
        assert_eq!(report.restored(), 3);
        assert_eq!(report.message(), "Restored 3 of 3 bulbs.");
    }
}
