use std::time::Instant;

use crate::bulb::health::{Health, HealthEvent, HealthLevel};
use crate::bulb::model::{Bulb, BulbState};
use crate::bulb::worker::{WorkerEvent, WorkerHandle};

#[derive(Debug)]
pub struct BulbRuntime {
    bulb: Bulb,
    state: BulbState,
    seen: bool,
    health: Health,
    edited_at: Option<Instant>,
    worker: Option<WorkerHandle>,
}

impl BulbRuntime {
    fn new(bulb: Bulb) -> Self {
        Self {
            bulb,
            state: BulbState::default(),
            seen: false,
            health: Health::default(),
            edited_at: None,
            worker: None,
        }
    }

    pub fn bulb(&self) -> &Bulb {
        &self.bulb
    }

    pub fn id(&self) -> &str {
        &self.bulb.id
    }

    pub fn name(&self) -> &str {
        &self.bulb.name
    }

    pub fn state(&self) -> &BulbState {
        &self.state
    }

    /// The state for a local control change. The edited values count as a known
    /// state even before the worker reports, so the deck does not fall back to
    /// the unknown placeholder after the user moves a slider.
    pub fn state_mut(&mut self) -> &mut BulbState {
        self.seen = true;
        &mut self.state
    }

    pub fn has_state(&self) -> bool {
        self.seen
    }

    pub fn status(&self) -> HealthLevel {
        self.health.level()
    }

    pub fn message(&self) -> Option<&str> {
        self.health.message()
    }

    pub fn is_unreachable(&self) -> bool {
        self.health.is_down()
    }

    pub fn is_run_unreachable(&self) -> bool {
        self.health.run_unreachable()
    }

    pub fn worker(&self) -> Option<&WorkerHandle> {
        self.worker.as_ref()
    }

    pub fn touch(&mut self) {
        self.edited_at = Some(Instant::now());
    }

    pub fn accepts_read_at(&self, observed_at: Instant) -> bool {
        self.edited_at
            .is_none_or(|edited_at| observed_at >= edited_at)
    }

    fn apply_health(&mut self, event: HealthEvent, in_run: bool) -> bool {
        self.health.advance(event, in_run)
    }

    fn end_run(&mut self) {
        self.health.end_run();
    }

    fn begin_run(&mut self) {
        self.health.begin_run();
    }

    fn merge_state(&mut self, patch: &BulbState, observed_at: Instant) -> Option<StateUpdate> {
        if !self.accepts_read_at(observed_at) {
            return None;
        }
        let was_on = self.state.power == Some(true);
        let before = self.state.clone();
        self.state.apply(patch);
        self.seen = true;
        Some(StateUpdate {
            changed: self.state != before,
            power_changed: was_on != (self.state.power == Some(true)),
        })
    }

    fn set_worker(&mut self, handle: WorkerHandle) {
        if let Some(previous) = self.worker.replace(handle) {
            previous.stop();
        }
        self.health = Health::default();
    }

    fn drop_worker(&mut self) -> bool {
        self.worker.take().is_some()
    }
}

impl Drop for BulbRuntime {
    fn drop(&mut self) {
        if let Some(worker) = &self.worker {
            worker.stop();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateUpdate {
    pub changed: bool,
    pub power_changed: bool,
}

#[derive(Default)]
pub struct Runtimes {
    records: Vec<BulbRuntime>,
}

impl Runtimes {
    pub fn from_bulbs(bulbs: Vec<Bulb>) -> Self {
        let mut runtimes = Self::default();
        for bulb in bulbs {
            runtimes.add(bulb);
        }
        runtimes
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &BulbRuntime> {
        self.records.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut BulbRuntime> {
        self.records.iter_mut()
    }

    pub fn first(&self) -> Option<&BulbRuntime> {
        self.records.first()
    }

    pub fn get(&self, id: &str) -> Option<&BulbRuntime> {
        self.records.iter().find(|runtime| runtime.id() == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut BulbRuntime> {
        self.records.iter_mut().find(|runtime| runtime.id() == id)
    }

    pub fn worker(&self, id: &str) -> Option<&WorkerHandle> {
        self.get(id)?.worker()
    }

    pub fn bulbs(&self) -> impl Iterator<Item = &Bulb> {
        self.records.iter().map(BulbRuntime::bulb)
    }

    pub fn bulb_list(&self) -> Vec<Bulb> {
        self.bulbs().cloned().collect()
    }

    pub fn add(&mut self, bulb: Bulb) {
        if let Some(runtime) = self.get_mut(&bulb.id) {
            runtime.bulb = bulb;
            return;
        }
        self.records.push(BulbRuntime::new(bulb));
    }

    pub fn update_bulb(&mut self, bulb: Bulb) {
        if let Some(runtime) = self.get_mut(&bulb.id) {
            runtime.bulb = bulb;
        }
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let Some(index) = self.records.iter().position(|runtime| runtime.id() == id) else {
            return false;
        };
        self.records.remove(index);
        true
    }

    /// Refresh keeps the state a still-configured bulb already reported, but
    /// rebuilds every other part of the record: a new worker, no edit stamp and
    /// a fresh health state.
    pub fn reload(&mut self, bulbs: Vec<Bulb>) {
        let previous = std::mem::take(&mut self.records);
        let mut records = Vec::with_capacity(bulbs.len());
        for bulb in bulbs {
            let mut runtime = BulbRuntime::new(bulb);
            if let Some(old) = previous.iter().find(|old| old.id() == runtime.id()) {
                runtime.state = old.state.clone();
                runtime.seen = old.seen;
            }
            records.push(runtime);
        }
        self.records = records;
    }

    pub fn clear(&mut self) {
        self.records.clear();
    }

    pub fn set_state(&mut self, id: &str, state: BulbState) -> bool {
        match self.get_mut(id) {
            Some(runtime) => {
                *runtime.state_mut() = state;
                true
            }
            None => false,
        }
    }

    pub fn touch(&mut self, id: &str) -> bool {
        match self.get_mut(id) {
            Some(runtime) => {
                runtime.touch();
                true
            }
            None => false,
        }
    }

    pub fn merge_state(
        &mut self,
        id: &str,
        patch: &BulbState,
        observed_at: Instant,
    ) -> Option<StateUpdate> {
        self.get_mut(id)?.merge_state(patch, observed_at)
    }

    pub fn apply_health(&mut self, id: &str, event: HealthEvent, in_run: bool) -> bool {
        self.get_mut(id)
            .is_some_and(|runtime| runtime.apply_health(event, in_run))
    }

    pub fn begin_run(&mut self) {
        for runtime in &mut self.records {
            runtime.begin_run();
        }
    }

    pub fn end_run(&mut self) {
        for runtime in &mut self.records {
            runtime.end_run();
        }
    }

    pub fn accepts(&self, event: &WorkerEvent) -> bool {
        self.get(event.id())
            .and_then(BulbRuntime::worker)
            .is_some_and(|worker| worker.is_current(event.worker()))
    }

    pub fn set_worker(&mut self, id: &str, handle: WorkerHandle) -> bool {
        match self.get_mut(id) {
            Some(runtime) => {
                runtime.set_worker(handle);
                true
            }
            None => {
                handle.stop();
                false
            }
        }
    }

    pub fn drop_worker(&mut self, id: &str) -> bool {
        self.get_mut(id).is_some_and(BulbRuntime::drop_worker)
    }

    pub fn stop_workers(&self) {
        for runtime in &self.records {
            if let Some(worker) = runtime.worker() {
                worker.stop();
            }
        }
    }

    pub fn unreachable_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .records
            .iter()
            .filter(|runtime| runtime.is_unreachable())
            .map(|runtime| runtime.id().to_owned())
            .collect();
        ids.sort();
        ids
    }
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;
    use std::sync::mpsc::channel;
    use std::time::{Duration, Instant};

    use super::Runtimes;
    use crate::bulb::health::{HealthEvent, HealthLevel};
    use crate::bulb::model::{Bulb, BulbState, ColorMode, Rgb};
    use crate::bulb::worker::{Command, WorkerEvent, test_handle};

    fn bulb(id: &str, name: &str) -> Bulb {
        Bulb {
            id: id.to_owned(),
            name: name.to_owned(),
            address: IpAddr::from([192, 168, 1, 42]),
            model: None,
        }
    }

    fn state() -> BulbState {
        BulbState {
            power: Some(true),
            brightness: Some(40),
            color: Some(Rgb::new(255, 0, 0)),
            mode: ColorMode::Color,
            ..BulbState::default()
        }
    }

    #[test]
    fn one_record_owns_the_bulb_state_health_edit_and_worker()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut runtimes = Runtimes::from_bulbs(vec![bulb("desk", "Desk lamp")]);
        let observed = Instant::now();

        let runtime = runtimes
            .get("desk")
            .ok_or("the configured bulb has a record")?;
        assert_eq!(runtime.bulb().name, "Desk lamp");
        assert!(!runtime.has_state());
        assert_eq!(runtime.status(), HealthLevel::Connecting);
        assert!(runtime.worker().is_none());
        assert!(runtime.accepts_read_at(observed));

        assert!(runtimes.touch("desk"));
        let runtime = runtimes
            .get("desk")
            .ok_or("the configured bulb has a record")?;
        assert!(!runtime.accepts_read_at(observed));

        let update = runtimes
            .merge_state("desk", &state(), Instant::now())
            .ok_or("the edit accepts a read after it")?;
        assert!(update.changed);
        assert!(update.power_changed);
        assert!(
            runtimes
                .get("desk")
                .is_some_and(|runtime| runtime.has_state())
        );

        assert!(runtimes.apply_health("desk", HealthEvent::Error("no answer".into()), false));
        let runtime = runtimes
            .get("desk")
            .ok_or("the configured bulb has a record")?;
        assert_eq!(runtime.status(), HealthLevel::Unreachable);
        assert_eq!(runtime.message(), Some("no answer"));

        let (commands, _inbox) = channel();
        assert!(runtimes.set_worker("desk", test_handle(commands)));
        assert!(
            runtimes
                .get("desk")
                .is_some_and(|runtime| runtime.worker().is_some())
        );
        Ok(())
    }

    #[test]
    fn removing_a_bulb_drops_the_record_and_stops_the_worker() {
        let mut runtimes =
            Runtimes::from_bulbs(vec![bulb("desk", "Desk lamp"), bulb("shelf", "Shelf lamp")]);
        let (commands, inbox) = channel();
        assert!(runtimes.set_worker("desk", test_handle(commands)));
        assert!(runtimes.touch("desk"));

        assert!(runtimes.remove("desk"));

        assert!(runtimes.get("desk").is_none());
        assert_eq!(runtimes.len(), 1);
        assert!(matches!(inbox.try_recv(), Ok(Command::Stop)));
    }

    #[test]
    fn refreshing_rebuilds_one_record_per_configured_bulb() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut runtimes =
            Runtimes::from_bulbs(vec![bulb("desk", "Desk lamp"), bulb("shelf", "Shelf lamp")]);
        let observed = Instant::now();
        assert!(runtimes.merge_state("desk", &state(), observed).is_some());
        assert!(runtimes.touch("desk"));
        let (commands, _inbox) = channel();
        assert!(runtimes.set_worker("desk", test_handle(commands)));

        runtimes.reload(vec![bulb("desk", "Desk lamp"), bulb("ceiling", "Ceiling")]);

        assert_eq!(runtimes.len(), 2);
        assert!(runtimes.get("shelf").is_none());
        assert!(runtimes.get("ceiling").is_some());
        let desk = runtimes.get("desk").ok_or("the kept bulb has a record")?;
        assert!(desk.has_state());
        assert_eq!(desk.status(), HealthLevel::Connecting);
        assert!(desk.worker().is_none());
        assert!(desk.accepts_read_at(observed));
        Ok(())
    }

    #[test]
    fn an_unconfigured_id_has_no_record() {
        let mut runtimes = Runtimes::from_bulbs(vec![bulb("desk", "Desk lamp")]);

        assert!(!runtimes.touch("missing"));
        assert!(
            runtimes
                .merge_state("missing", &state(), Instant::now())
                .is_none()
        );
        assert!(!runtimes.apply_health("missing", HealthEvent::Sent, false));
        assert!(!runtimes.set_worker("missing", test_handle(channel().0)));

        runtimes.remove("desk");
        assert!(runtimes.is_empty());
        assert!(runtimes.get("desk").is_none());
    }

    #[test]
    fn a_stale_worker_event_is_ignored() {
        let mut runtimes = Runtimes::from_bulbs(vec![bulb("desk", "Desk lamp")]);
        let (commands, _inbox) = channel();
        assert!(runtimes.set_worker("desk", test_handle(commands)));

        let current = WorkerEvent::Sent {
            worker: 0,
            id: "desk".to_owned(),
        };
        let stale = WorkerEvent::Sent {
            worker: 7,
            id: "desk".to_owned(),
        };
        assert!(runtimes.accepts(&current));
        assert!(!runtimes.accepts(&stale));

        runtimes.remove("desk");
        assert!(!runtimes.accepts(&current));
    }

    #[test]
    fn the_edit_stamp_accepts_a_read_after_it() -> Result<(), Box<dyn std::error::Error>> {
        let mut runtimes = Runtimes::from_bulbs(vec![bulb("desk", "Desk lamp")]);
        let observed = Instant::now();
        runtimes.touch("desk");

        let runtime = runtimes
            .get("desk")
            .ok_or("the configured bulb has a record")?;
        assert!(runtime.accepts_read_at(observed + Duration::from_millis(1)));
        Ok(())
    }

    #[test]
    fn a_run_end_clears_only_the_run_mark_on_the_record() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut runtimes = Runtimes::from_bulbs(vec![bulb("desk", "Desk lamp")]);
        for message in ["one", "two", "three"] {
            runtimes.apply_health("desk", HealthEvent::Error(message.into()), true);
        }

        runtimes.end_run();

        let runtime = runtimes
            .get("desk")
            .ok_or("the configured bulb has a record")?;
        assert!(runtime.is_unreachable());
        assert!(!runtime.is_run_unreachable());
        Ok(())
    }

    #[test]
    fn a_new_worker_resets_a_failed_health() -> Result<(), Box<dyn std::error::Error>> {
        let mut runtimes = Runtimes::from_bulbs(vec![bulb("desk", "Desk lamp")]);
        runtimes.apply_health("desk", HealthEvent::Exited, false);
        let runtime = runtimes
            .get("desk")
            .ok_or("the configured bulb has a record")?;
        assert_eq!(runtime.status(), HealthLevel::Failed);
        assert!(runtime.message().is_some());

        let (commands, _inbox) = channel();
        assert!(runtimes.set_worker("desk", test_handle(commands)));

        let runtime = runtimes
            .get("desk")
            .ok_or("the configured bulb has a record")?;
        assert_eq!(runtime.status(), HealthLevel::Connecting);
        assert_eq!(runtime.message(), None);
        Ok(())
    }

    #[test]
    fn unreachable_ids_lists_standing_and_run_failures_in_id_order() {
        let mut runtimes =
            Runtimes::from_bulbs(vec![bulb("desk", "Desk lamp"), bulb("shelf", "Shelf lamp")]);
        assert!(runtimes.unreachable_ids().is_empty());

        runtimes.apply_health("desk", HealthEvent::Error("one".into()), true);
        assert!(runtimes.unreachable_ids().is_empty());

        runtimes.apply_health("shelf", HealthEvent::Error("no answer".into()), false);
        for message in ["one", "two", "three"] {
            runtimes.apply_health("desk", HealthEvent::Error(message.into()), true);
        }

        assert_eq!(
            runtimes.unreachable_ids(),
            vec!["desk".to_owned(), "shelf".to_owned()]
        );
    }

    #[test]
    fn adding_an_existing_id_updates_metadata_and_keeps_the_live_record()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut runtimes = Runtimes::from_bulbs(vec![bulb("desk", "Desk lamp")]);
        assert!(
            runtimes
                .merge_state("desk", &state(), Instant::now())
                .is_some()
        );
        let (commands, inbox) = channel();
        assert!(runtimes.set_worker("desk", test_handle(commands)));
        runtimes.apply_health("desk", HealthEvent::Error("no answer".into()), false);

        runtimes.add(bulb("desk", "Renamed lamp"));

        assert_eq!(runtimes.len(), 1);
        let runtime = runtimes
            .get("desk")
            .ok_or("the configured bulb has a record")?;
        assert_eq!(runtime.name(), "Renamed lamp");
        assert!(runtime.has_state());
        assert_eq!(runtime.status(), HealthLevel::Unreachable);
        assert_eq!(runtime.message(), Some("no answer"));
        assert!(runtime.worker().is_some());
        assert!(inbox.try_recv().is_err());
        Ok(())
    }
}
