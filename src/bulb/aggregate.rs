use std::collections::HashMap;

use crate::bulb::health::HealthSet;
use crate::bulb::model::{Bulb, BulbState, ColorMode, Rgb};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValueSpan<T> {
    pub low: T,
    pub high: T,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AggregatePower {
    On,
    Off,
    Mixed,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightQuality {
    White,
    Color,
    Mixed,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AggregateColor {
    Equal(Rgb),
    Mixed,
}

#[derive(Clone, Debug)]
pub struct AggregateState {
    pub total: usize,
    pub reachable: usize,
    pub failed: usize,
    pub power: AggregatePower,
    pub quality: LightQuality,
    pub brightness: Option<ValueSpan<u8>>,
    pub temperature: Option<ValueSpan<u16>>,
    pub color: Option<AggregateColor>,
}

pub fn light_quality_of(mode: ColorMode) -> LightQuality {
    match mode {
        ColorMode::Temperature => LightQuality::White,
        ColorMode::Color => LightQuality::Color,
        ColorMode::Unknown => LightQuality::Unknown,
    }
}

pub fn power_of_state(state: &BulbState) -> AggregatePower {
    match state.power {
        Some(true) => AggregatePower::On,
        Some(false) => AggregatePower::Off,
        None => AggregatePower::Unknown,
    }
}

pub fn aggregate_state(
    bulbs: &[Bulb],
    states: &HashMap<String, BulbState>,
    health: &HealthSet,
) -> AggregateState {
    let mut powers: Vec<Option<bool>> = Vec::new();
    let mut qualities: Vec<Option<LightQuality>> = Vec::new();
    let mut brightness = Vec::new();
    let mut temperatures = Vec::new();
    let mut colors = Vec::new();
    let mut reachable = 0usize;

    for bulb in bulbs {
        let Some(state) = states.get(&bulb.id) else {
            continue;
        };
        if health.unreachable(&bulb.id) {
            continue;
        }
        reachable += 1;
        let quality = light_quality_of(state.mode);
        qualities.push((quality != LightQuality::Unknown).then_some(quality));
        powers.push(state.power);
        if let Some(value) = state.brightness {
            brightness.push(value);
        }
        if let Some(value) = state.temperature {
            temperatures.push(value);
        }
        if let Some(value) = state.color {
            colors.push(value);
        }
    }

    AggregateState {
        total: bulbs.len(),
        reachable,
        failed: bulbs
            .iter()
            .filter(|bulb| health.unreachable(&bulb.id))
            .count(),
        power: power_of(&powers),
        quality: quality_of(&qualities),
        brightness: span_of(&brightness),
        temperature: span_of(&temperatures),
        color: color_of(&colors),
    }
}

fn span_of<T: Copy + Ord>(values: &[T]) -> Option<ValueSpan<T>> {
    let mut span: Option<ValueSpan<T>> = None;
    for value in values {
        span = match span {
            Some(current) => Some(ValueSpan {
                low: current.low.min(*value),
                high: current.high.max(*value),
            }),
            None => Some(ValueSpan {
                low: *value,
                high: *value,
            }),
        };
    }
    span
}

fn power_of(values: &[Option<bool>]) -> AggregatePower {
    if values.is_empty() || values.iter().all(|value| value.is_none()) {
        return AggregatePower::Unknown;
    }
    if values.iter().any(|value| value.is_none()) {
        return AggregatePower::Mixed;
    }
    let on = values.iter().filter(|value| **value == Some(true)).count();
    if on == values.len() {
        AggregatePower::On
    } else if on == 0 {
        AggregatePower::Off
    } else {
        AggregatePower::Mixed
    }
}

fn quality_of(values: &[Option<LightQuality>]) -> LightQuality {
    let Some(first) = values.first() else {
        return LightQuality::Unknown;
    };
    if values.iter().all(|value| value == first) {
        (*first).unwrap_or(LightQuality::Unknown)
    } else {
        LightQuality::Mixed
    }
}

fn color_of(values: &[Rgb]) -> Option<AggregateColor> {
    let first = *values.first()?;
    if values.iter().all(|value| *value == first) {
        Some(AggregateColor::Equal(first))
    } else {
        Some(AggregateColor::Mixed)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::net::IpAddr;

    use super::*;
    use crate::bulb::health::{HealthEvent, HealthSet};
    use crate::bulb::model::{Bulb, BulbState, ColorMode, Rgb};

    fn bulb(id: &str, name: &str) -> Bulb {
        Bulb {
            id: id.to_owned(),
            name: name.to_owned(),
            address: IpAddr::from([192, 168, 1, 10]),
            model: None,
        }
    }

    #[test]
    fn aggregate_reports_mixed_power_and_spans() {
        let bulbs = vec![bulb("a", "A"), bulb("b", "B")];
        let mut states = HashMap::new();
        states.insert(
            "a".to_owned(),
            BulbState {
                power: Some(true),
                brightness: Some(40),
                temperature: Some(3_000),
                mode: ColorMode::Temperature,
                ..Default::default()
            },
        );
        states.insert(
            "b".to_owned(),
            BulbState {
                power: Some(false),
                brightness: Some(80),
                temperature: Some(4_000),
                color: Some(Rgb::new(255, 0, 0)),
                mode: ColorMode::Color,
            },
        );

        let aggregate = aggregate_state(&bulbs, &states, &HealthSet::default());
        assert_eq!(aggregate.power, AggregatePower::Mixed);
        assert_eq!(aggregate.quality, LightQuality::Mixed);
        assert_eq!(aggregate.brightness, Some(ValueSpan { low: 40, high: 80 }));
        assert_eq!(
            aggregate.temperature,
            Some(ValueSpan {
                low: 3_000,
                high: 4_000
            })
        );
    }

    #[test]
    fn the_stage_count_matches_the_picker_health() {
        let bulbs = vec![bulb("a", "A"), bulb("b", "B"), bulb("c", "C")];
        let mut states = HashMap::new();
        states.insert("a".to_owned(), BulbState::default());
        states.insert("c".to_owned(), BulbState::default());
        let mut health = HealthSet::default();
        health.apply("b", HealthEvent::Error("no answer".to_owned()));
        health.apply("c", HealthEvent::Error("no answer".to_owned()));

        let aggregate = aggregate_state(&bulbs, &states, &health);

        let picker = bulbs
            .iter()
            .filter(|bulb| health.unreachable(&bulb.id))
            .count();
        assert_eq!(aggregate.failed, picker);
        assert_eq!(aggregate.failed, 2);
        assert_eq!(aggregate.reachable, 1);
    }

    #[test]
    fn an_unknown_power_value_is_mixed() {
        let bulbs = vec![bulb("a", "A"), bulb("b", "B")];
        let states = HashMap::from([
            (
                "a".to_owned(),
                BulbState {
                    power: Some(true),
                    ..BulbState::default()
                },
            ),
            ("b".to_owned(), BulbState::default()),
        ]);

        let aggregate = aggregate_state(&bulbs, &states, &HealthSet::default());

        assert_eq!(aggregate.power, AggregatePower::Mixed);
    }

    #[test]
    fn an_unknown_mode_is_mixed() {
        let bulbs = vec![bulb("a", "A"), bulb("b", "B")];
        let states = HashMap::from([
            (
                "a".to_owned(),
                BulbState {
                    mode: ColorMode::Temperature,
                    ..BulbState::default()
                },
            ),
            ("b".to_owned(), BulbState::default()),
        ]);

        let aggregate = aggregate_state(&bulbs, &states, &HealthSet::default());

        assert_eq!(aggregate.quality, LightQuality::Mixed);
    }

    #[test]
    fn an_all_unknown_aggregate_stays_unknown() {
        let bulbs = vec![bulb("a", "A")];
        let states = HashMap::from([("a".to_owned(), BulbState::default())]);

        let aggregate = aggregate_state(&bulbs, &states, &HealthSet::default());

        assert_eq!(aggregate.power, AggregatePower::Unknown);
        assert_eq!(aggregate.quality, LightQuality::Unknown);
    }
}
