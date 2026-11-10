use std::collections::HashMap;

use crate::bulb::aggregate::{
    AggregateColor, AggregatePower, LightQuality, ValueSpan, aggregate_state, light_quality_of,
    power_of_state,
};
use crate::bulb::health::{HealthSet, HealthStatus};
use crate::bulb::model::{Bulb, BulbState, Rgb};
use crate::bulb::selection::Selection;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectedScope {
    All,
    Bulb,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectedHealth {
    pub total: usize,
    pub reachable: usize,
    pub failed: usize,
    pub status: Option<HealthStatus>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectedLight {
    pub scope: SelectedScope,
    pub name: String,
    pub power: AggregatePower,
    pub quality: LightQuality,
    pub brightness: Option<ValueSpan<u8>>,
    pub temperature: Option<ValueSpan<u16>>,
    pub color: Option<AggregateColor>,
    pub health: SelectedHealth,
}

impl SelectedLight {
    pub fn is_all(&self) -> bool {
        matches!(self.scope, SelectedScope::All)
    }

    pub fn scope_label(&self) -> String {
        if self.is_all() {
            format!("{} bulbs", self.health.total)
        } else {
            self.name.clone()
        }
    }

    pub fn brightness_value(&self) -> u8 {
        self.brightness.map_or(50, |span| span.high)
    }

    pub fn brightness_text(&self) -> String {
        span_label(self.brightness, self.brightness_value(), "")
    }

    pub fn brightness_label(&self) -> String {
        span_label(self.brightness, self.brightness_value(), "%")
    }

    pub fn temperature_value(&self) -> u16 {
        self.temperature.map_or(4_000, |span| span.high)
    }

    pub fn temperature_label(&self) -> String {
        span_label(self.temperature, self.temperature_value(), " K")
    }

    pub fn color_value(&self) -> Option<Rgb> {
        match self.color {
            Some(AggregateColor::Equal(color)) => Some(color),
            _ => None,
        }
    }

    pub fn color_mixed(&self) -> bool {
        matches!(self.color, Some(AggregateColor::Mixed))
    }
}

fn span_label<T: std::fmt::Display + PartialEq>(
    span: Option<ValueSpan<T>>,
    value: T,
    suffix: &str,
) -> String {
    match span {
        Some(span) if span.low != span.high => format!("{}–{}{suffix}", span.low, span.high),
        _ => format!("{value}{suffix}"),
    }
}

pub fn selected_light(
    bulbs: &[Bulb],
    states: &HashMap<String, BulbState>,
    health: &HealthSet,
    selection: Option<&Selection>,
) -> Option<SelectedLight> {
    match selection {
        Some(Selection::All) => {
            let aggregate = aggregate_state(bulbs, states, health);
            Some(SelectedLight {
                scope: SelectedScope::All,
                name: "All bulbs".to_owned(),
                power: aggregate.power,
                quality: aggregate.quality,
                brightness: aggregate.brightness,
                temperature: aggregate.temperature,
                color: aggregate.color,
                health: SelectedHealth {
                    total: aggregate.total,
                    reachable: aggregate.reachable,
                    failed: aggregate.failed,
                    status: None,
                    message: bulbs
                        .iter()
                        .find_map(|bulb| health.message(&bulb.id))
                        .map(str::to_owned),
                },
            })
        }
        Some(Selection::Bulb(id)) => {
            let bulb = bulbs.iter().find(|bulb| &bulb.id == id)?;
            let state = states.get(id).cloned().unwrap_or_default();
            let answered = states.contains_key(id);
            let status = health.status(id);
            let down = status.is_down();
            Some(SelectedLight {
                scope: SelectedScope::Bulb,
                name: bulb.name.clone(),
                power: power_of_state(&state),
                quality: light_quality_of(state.mode),
                brightness: state.brightness.map(|value| ValueSpan {
                    low: value,
                    high: value,
                }),
                temperature: state.temperature.map(|value| ValueSpan {
                    low: value,
                    high: value,
                }),
                color: state.color.map(AggregateColor::Equal),
                health: SelectedHealth {
                    total: 1,
                    reachable: if answered && !down { 1 } else { 0 },
                    failed: if down { 1 } else { 0 },
                    status: Some(status),
                    message: health.message(id).map(str::to_owned),
                },
            })
        }
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::net::IpAddr;

    use super::{SelectedScope, selected_light};
    use crate::bulb::aggregate::{AggregateColor, AggregatePower, LightQuality};
    use crate::bulb::health::{HealthEvent, HealthSet};
    use crate::bulb::model::{Bulb, BulbState, ColorMode, Rgb};
    use crate::bulb::selection::Selection;

    fn bulb(id: &str, name: &str) -> Bulb {
        Bulb {
            id: id.to_owned(),
            name: name.to_owned(),
            address: IpAddr::from([192, 168, 1, 42]),
            model: None,
        }
    }

    fn state(on: bool, mode: ColorMode, brightness: u8, temperature: u16) -> BulbState {
        BulbState {
            power: Some(on),
            brightness: Some(brightness),
            temperature: Some(temperature),
            color: Some(Rgb::new(255, 0, 0)),
            mode,
        }
    }

    fn pair() -> (Vec<Bulb>, HashMap<String, BulbState>) {
        let bulbs = vec![bulb("a", "Desk"), bulb("b", "Shelf")];
        let state = state(true, ColorMode::Temperature, 40, 3_000);
        let states = HashMap::from([("a".to_owned(), state.clone()), ("b".to_owned(), state)]);
        (bulbs, states)
    }

    #[test]
    fn all_bulbs_and_one_bulb_produce_the_same_view() -> Result<(), Box<dyn std::error::Error>> {
        let (bulbs, states) = pair();
        let health = HealthSet::default();

        let all = selected_light(&bulbs, &states, &health, Some(&Selection::All))
            .ok_or("the all view should exist")?;
        let one = selected_light(
            &bulbs,
            &states,
            &health,
            Some(&Selection::Bulb("a".to_owned())),
        )
        .ok_or("the single view should exist")?;

        assert_eq!(all.scope, SelectedScope::All);
        assert_eq!(one.scope, SelectedScope::Bulb);
        assert_eq!(all.power, one.power);
        assert_eq!(all.quality, one.quality);
        assert_eq!(all.brightness, one.brightness);
        assert_eq!(all.temperature, one.temperature);
        assert_eq!(all.color, one.color);
        assert_eq!(all.brightness_value(), one.brightness_value());
        assert_eq!(all.temperature_value(), one.temperature_value());
        assert_eq!(all.scope_label(), "2 bulbs");
        assert_eq!(one.scope_label(), "Desk");
        Ok(())
    }

    #[test]
    fn mixed_quality_and_colour_render_one_clear_state() -> Result<(), Box<dyn std::error::Error>> {
        let (bulbs, mut states) = pair();
        if let Some(shelf) = states.get_mut("b") {
            *shelf = state(true, ColorMode::Color, 80, 4_000);
            shelf.color = Some(Rgb::new(0, 255, 0));
        }
        let health = HealthSet::default();

        let all = selected_light(&bulbs, &states, &health, Some(&Selection::All))
            .ok_or("the all view should exist")?;

        assert_eq!(all.quality, LightQuality::Mixed);
        assert_eq!(all.color, Some(AggregateColor::Mixed));
        assert!(all.color_mixed());
        assert_eq!(all.color_value(), None);
        assert_eq!(all.brightness_label(), "40–80%");
        assert_eq!(all.temperature_label(), "3000–4000 K");
        Ok(())
    }

    #[test]
    fn an_unknown_state_keeps_the_control_defaults() -> Result<(), Box<dyn std::error::Error>> {
        let bulbs = vec![bulb("a", "Desk")];
        let states = HashMap::new();
        let health = HealthSet::default();

        let view = selected_light(
            &bulbs,
            &states,
            &health,
            Some(&Selection::Bulb("a".to_owned())),
        )
        .ok_or("the single view should exist")?;

        assert_eq!(view.power, AggregatePower::Unknown);
        assert_eq!(view.quality, LightQuality::Unknown);
        assert_eq!(view.brightness, None);
        assert_eq!(view.temperature, None);
        assert_eq!(view.color, None);
        assert_eq!(view.brightness_value(), 50);
        assert_eq!(view.brightness_text(), "50");
        assert_eq!(view.brightness_label(), "50%");
        assert_eq!(view.temperature_value(), 4_000);
        assert_eq!(view.temperature_label(), "4000 K");
        assert_eq!(view.health.total, 1);
        assert_eq!(view.health.reachable, 0);
        assert_eq!(view.health.failed, 0);
        Ok(())
    }

    #[test]
    fn an_unknown_all_view_matches_the_one_view() -> Result<(), Box<dyn std::error::Error>> {
        let bulbs = vec![bulb("a", "Desk"), bulb("b", "Shelf")];
        let states = HashMap::new();
        let health = HealthSet::default();

        let all = selected_light(&bulbs, &states, &health, Some(&Selection::All))
            .ok_or("the all view should exist")?;

        assert_eq!(all.power, AggregatePower::Unknown);
        assert_eq!(all.quality, LightQuality::Unknown);
        assert_eq!(all.brightness, None);
        assert_eq!(all.temperature, None);
        assert_eq!(all.color, None);
        assert_eq!(all.health.total, 2);
        assert_eq!(all.health.reachable, 0);
        assert_eq!(all.health.failed, 0);
        assert_eq!(all.brightness_value(), 50);
        assert_eq!(all.temperature_value(), 4_000);
        Ok(())
    }

    #[test]
    fn the_view_reports_one_unreachable_count() -> Result<(), Box<dyn std::error::Error>> {
        let bulbs = vec![bulb("a", "Desk"), bulb("b", "Shelf"), bulb("c", "Ceiling")];
        let mut states = HashMap::new();
        states.insert(
            "a".to_owned(),
            state(true, ColorMode::Temperature, 40, 3_000),
        );
        states.insert(
            "c".to_owned(),
            state(true, ColorMode::Temperature, 40, 3_000),
        );
        let mut health = HealthSet::default();
        health.apply("b", HealthEvent::Error("no answer".to_owned()), false);
        health.apply("c", HealthEvent::Error("no answer".to_owned()), false);

        let all = selected_light(&bulbs, &states, &health, Some(&Selection::All))
            .ok_or("the all view should exist")?;
        let shelf = selected_light(
            &bulbs,
            &states,
            &health,
            Some(&Selection::Bulb("b".to_owned())),
        )
        .ok_or("the single view should exist")?;

        assert_eq!(all.health.total, 3);
        assert_eq!(all.health.reachable, 1);
        assert_eq!(all.health.failed, 2);
        assert_eq!(all.health.message.as_deref(), Some("no answer"));
        assert_eq!(shelf.health.total, 1);
        assert_eq!(shelf.health.failed, 1);
        assert_eq!(shelf.health.reachable, 0);
        assert!(shelf.health.status.is_some_and(|status| status.is_down()));
        Ok(())
    }

    #[test]
    fn a_missing_selection_has_no_view() {
        let (bulbs, states) = pair();
        let health = HealthSet::default();

        assert!(selected_light(&bulbs, &states, &health, None).is_none());
        assert!(
            selected_light(
                &bulbs,
                &states,
                &health,
                Some(&Selection::Bulb("missing".to_owned()))
            )
            .is_none()
        );
    }
}
