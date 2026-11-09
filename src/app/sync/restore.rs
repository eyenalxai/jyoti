use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::bulb::model::{Bulb, BulbState, Rgb};
use crate::screen::sync::{RestoreControl, RestoreReport, restore_control};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Writes {
    color: bool,
    brightness: bool,
}

#[derive(Clone, Debug)]
struct Snapshot {
    name: String,
    state: BulbState,
}

#[derive(Clone, Debug, Default)]
pub struct RestorePlan {
    expected: BTreeMap<String, Snapshot>,
    driven: BTreeMap<String, Writes>,
    waiting: BTreeSet<String>,
    acknowledged: BTreeSet<String>,
    failed: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestoreControls {
    pub color: Option<Rgb>,
    pub temperature: Option<u16>,
    pub brightness: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RestoreRequest {
    Restore {
        id: String,
        controls: RestoreControls,
    },
    Missing {
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
            if let Some(state) = states.get(&bulb.id) {
                self.observe(&bulb.id, &bulb.name, state);
            }
        }
    }

    pub fn observe(&mut self, id: &str, name: &str, state: &BulbState) -> bool {
        if self.expected.contains_key(id) {
            return false;
        }
        self.expected.insert(
            id.to_owned(),
            Snapshot {
                name: name.to_owned(),
                state: state.clone(),
            },
        );
        true
    }

    pub fn covers(&self, id: &str) -> bool {
        self.expected.contains_key(id)
    }

    pub fn record(&mut self, color: &[String], brightness: &[String]) {
        for id in color {
            self.record_one(id, true, false);
        }
        for id in brightness {
            self.record_one(id, false, true);
        }
    }

    fn record_one(&mut self, id: &str, color: bool, brightness: bool) {
        if !self.covers(id) {
            return;
        }
        let writes = self.driven.entry(id.to_owned()).or_default();
        writes.color |= color;
        writes.brightness |= brightness;
    }

    pub fn requests(&self) -> Vec<RestoreRequest> {
        self.driven
            .iter()
            .filter(|(id, _)| !self.settled(id))
            .map(|(id, writes)| match self.controls(id, *writes) {
                Some(controls) => RestoreRequest::Restore {
                    id: id.clone(),
                    controls,
                },
                None => RestoreRequest::Missing { id: id.clone() },
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

    pub fn report(&self) -> RestoreReport {
        let total = self.driven.len();
        let mut not_restored: Vec<String> = self
            .driven
            .iter()
            .filter(|(id, writes)| {
                !self.acknowledged.contains(*id) || self.controls(id, **writes).is_none()
            })
            .map(|(id, _)| {
                self.expected
                    .get(id)
                    .map_or_else(|| id.clone(), |snapshot| snapshot.name.clone())
            })
            .collect();
        not_restored.sort();
        RestoreReport {
            restored: total.saturating_sub(not_restored.len()),
            not_restored,
            total,
        }
    }

    fn settled(&self, id: &str) -> bool {
        self.waiting.contains(id) || self.acknowledged.contains(id) || self.failed.contains(id)
    }

    fn controls(&self, id: &str, writes: Writes) -> Option<RestoreControls> {
        let snapshot = self.expected.get(id)?;
        let mut color = None;
        let mut temperature = None;
        if writes.color {
            match restore_control(&snapshot.state) {
                Some(RestoreControl::Color(value)) => color = Some(value),
                Some(RestoreControl::Temperature(value)) => temperature = Some(value),
                None => return None,
            }
        }
        let brightness = if writes.brightness {
            Some(snapshot.state.brightness?)
        } else {
            None
        };
        Some(RestoreControls {
            color,
            temperature,
            brightness,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{RestoreControls, RestorePlan, RestoreRequest};
    use crate::bulb::model::{Bulb, BulbState, ColorMode, Rgb};

    fn the_restore_controls(plan: &RestorePlan) -> RestoreControls {
        let mut requests = plan.requests().into_iter();
        let request = requests.next();
        assert!(matches!(request, Some(RestoreRequest::Restore { .. })));
        let Some(RestoreRequest::Restore { controls, .. }) = request else {
            return RestoreControls {
                color: None,
                temperature: None,
                brightness: None,
            };
        };
        assert!(requests.next().is_none());
        controls
    }

    fn the_missing_id(plan: &RestorePlan) -> String {
        let mut requests = plan.requests().into_iter();
        let request = requests.next();
        assert!(matches!(request, Some(RestoreRequest::Missing { .. })));
        let Some(RestoreRequest::Missing { id }) = request else {
            return String::new();
        };
        assert!(requests.next().is_none());
        id
    }

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

    #[test]
    fn a_configured_bulb_joins_only_when_its_state_is_known() {
        let desk = bulb("desk", "Desk lamp");
        let shelf = bulb("shelf", "Shelf lamp");
        let states = HashMap::from([(desk.id.clone(), colour_state(Rgb::new(255, 0, 0), 40))]);
        let mut plan = RestorePlan::default();

        plan.begin(&[desk, shelf.clone()], &states);

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
        plan.record(&["bulb".to_owned()], &[]);

        let controls = the_restore_controls(&plan);
        assert_eq!(controls.color, Some(Rgb::new(255, 0, 0)));
        assert_eq!(controls.temperature, None);
        assert_eq!(controls.brightness, None);
    }

    #[test]
    fn a_run_that_never_published_has_nothing_to_restore() {
        let desk = bulb("desk", "Desk lamp");
        let states = HashMap::from([(desk.id.clone(), colour_state(Rgb::new(255, 0, 0), 40))]);
        let mut plan = RestorePlan::default();

        plan.begin(&[desk], &states);

        assert!(plan.requests().is_empty());
        assert!(!plan.is_open());
        let report = plan.report();
        assert_eq!(report.total, 0);
        assert_eq!(report.not_restored, Vec::<String>::new());
        assert_eq!(report.message(), "Nothing to restore.");
    }

    #[test]
    fn only_the_controls_the_run_wrote_are_restored() {
        let desk = bulb("desk", "Desk lamp");
        let states = HashMap::from([(desk.id.clone(), colour_state(Rgb::new(255, 128, 0), 40))]);
        let mut plan = RestorePlan::default();
        plan.begin(&[desk], &states);

        plan.record(&["desk".to_owned()], &[]);

        let controls = the_restore_controls(&plan);
        assert_eq!(controls.color, Some(Rgb::new(255, 128, 0)));
        assert_eq!(controls.temperature, None);
        assert_eq!(controls.brightness, None);
    }

    #[test]
    fn a_brightness_write_restores_only_brightness() {
        let desk = bulb("desk", "Desk lamp");
        let states = HashMap::from([(desk.id.clone(), colour_state(Rgb::new(255, 128, 0), 40))]);
        let mut plan = RestorePlan::default();
        plan.begin(&[desk], &states);

        plan.record(&[], &["desk".to_owned()]);

        let controls = the_restore_controls(&plan);
        assert_eq!(controls.color, None);
        assert_eq!(controls.temperature, None);
        assert_eq!(controls.brightness, Some(40));
    }

    #[test]
    fn a_temperature_snapshot_restores_the_temperature() {
        let desk = bulb("desk", "Desk lamp");
        let states = HashMap::from([(desk.id.clone(), temperature_state(3_000, 40))]);
        let mut plan = RestorePlan::default();
        plan.begin(&[desk], &states);

        plan.record(&["desk".to_owned()], &[]);

        let controls = the_restore_controls(&plan);
        assert_eq!(controls.color, None);
        assert_eq!(controls.temperature, Some(3_000));
        assert_eq!(controls.brightness, None);
    }

    #[test]
    fn a_missing_restore_value_is_named_as_not_restored() {
        let desk = bulb("desk", "Desk lamp");
        let states = HashMap::from([(desk.id.clone(), BulbState::default())]);
        let mut plan = RestorePlan::default();
        plan.begin(&[desk], &states);
        plan.record(&["desk".to_owned()], &[]);

        let id = the_missing_id(&plan);
        assert_eq!(id, "desk");
        plan.mark_failed(&id);

        let report = plan.report();
        assert_eq!(report.total, 1);
        assert_eq!(report.restored, 0);
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
        plan.record(
            &["desk".to_owned(), "shelf".to_owned()],
            &["desk".to_owned(), "shelf".to_owned()],
        );

        let requests = plan.requests();
        assert_eq!(requests.len(), 2);
        for request in &requests {
            let id = match request {
                RestoreRequest::Restore { id, .. } | RestoreRequest::Missing { id } => id,
            };
            plan.mark_waiting(id);
        }
        assert!(plan.is_open());

        plan.acknowledge("desk", true);
        assert!(plan.is_open());

        plan.acknowledge("shelf", true);
        assert!(!plan.is_open());

        let report = plan.report();
        assert_eq!(report.total, 2);
        assert_eq!(report.restored, 2);
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
        plan.record(
            &["desk".to_owned(), "shelf".to_owned()],
            &["desk".to_owned(), "shelf".to_owned()],
        );
        plan.mark_waiting("desk");
        plan.mark_waiting("shelf");
        plan.acknowledge("desk", true);

        let report = plan.report();
        assert_eq!(report.total, 2);
        assert_eq!(report.restored, 1);
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
        plan.record(&["desk".to_owned()], &[]);
        plan.mark_waiting("desk");

        plan.acknowledge("desk", false);

        assert!(!plan.is_open());
        let report = plan.report();
        assert_eq!(report.restored, 0);
        assert_eq!(report.not_restored, vec!["Desk lamp".to_owned()]);
    }

    #[test]
    fn the_report_counts_only_the_bulbs_the_run_drove() {
        let desk = bulb("desk", "Desk lamp");
        let shelf = bulb("shelf", "Shelf lamp");
        let states = HashMap::from([
            (desk.id.clone(), colour_state(Rgb::new(255, 0, 0), 40)),
            (shelf.id.clone(), colour_state(Rgb::new(0, 255, 0), 70)),
        ]);
        let mut plan = RestorePlan::default();
        plan.begin(&[desk, shelf], &states);
        plan.record(&["desk".to_owned()], &[]);
        plan.mark_waiting("desk");
        plan.acknowledge("desk", true);

        let report = plan.report();
        assert_eq!(report.total, 1);
        assert_eq!(report.restored, 1);
        assert_eq!(report.message(), "Restored 1 of 1 bulbs.");
    }
}
