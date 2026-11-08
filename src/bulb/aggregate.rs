use std::collections::HashMap;

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

pub fn aggregate_state(
    bulbs: &[Bulb],
    states: &HashMap<String, BulbState>,
    errors: &HashMap<String, String>,
) -> AggregateState {
    let mut powers = Vec::new();
    let mut qualities = Vec::new();
    let mut brightness = Vec::new();
    let mut temperatures = Vec::new();
    let mut colors = Vec::new();
    let mut reachable = 0usize;
    let mut failed = 0usize;

    for bulb in bulbs {
        let Some(state) = states.get(&bulb.id) else {
            if errors.contains_key(&bulb.id) {
                failed += 1;
            }
            continue;
        };
        reachable += 1;
        let quality = light_quality_of(state.mode);
        if quality != LightQuality::Unknown {
            qualities.push(quality);
        }
        if let Some(power) = state.power {
            powers.push(power);
        }
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
        failed,
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

fn power_of(values: &[bool]) -> AggregatePower {
    if values.is_empty() {
        return AggregatePower::Unknown;
    }
    let on = values.iter().filter(|value| **value).count();
    if on == values.len() {
        AggregatePower::On
    } else if on == 0 {
        AggregatePower::Off
    } else {
        AggregatePower::Mixed
    }
}

fn quality_of(values: &[LightQuality]) -> LightQuality {
    let Some(first) = values.first() else {
        return LightQuality::Unknown;
    };
    if values.iter().all(|value| value == first) {
        *first
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
    use super::*;
    use crate::bulb::model::{Bulb, BulbState, ColorMode, Rgb};
    use std::collections::HashMap;

    fn bulb(id: &str, name: &str) -> Bulb {
        Bulb {
            id: id.to_owned(),
            name: name.to_owned(),
            address: "192.168.1.10".to_owned(),
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

        let aggregate = aggregate_state(&bulbs, &states, &HashMap::new());
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
}
