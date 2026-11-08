use serde_json::Value;

use crate::bulb::model::{
    BulbState, ColorMode, PIID_BRIGHTNESS, PIID_COLOR, PIID_COLOR_MODE, PIID_POWER,
    PIID_TEMPERATURE, Rgb, SIID_LIGHT, TEMPERATURE_MAX, TEMPERATURE_MIN,
};
use crate::miio::properties::{PropertyParam, properties};
use crate::miio::reply::MiioReply;

const STATE_PIIDS: [u32; 5] = [
    PIID_POWER,
    PIID_BRIGHTNESS,
    PIID_TEMPERATURE,
    PIID_COLOR,
    PIID_COLOR_MODE,
];

pub fn state_properties() -> Vec<PropertyParam> {
    properties(&STATE_PIIDS)
}

pub fn state_from_reply(reply: &MiioReply) -> BulbState {
    BulbState {
        power: as_boolean(read_value(reply, PIID_POWER)),
        brightness: as_integer(read_value(reply, PIID_BRIGHTNESS), 1, 100).map(|value| value as u8),
        temperature: as_integer(read_value(reply, PIID_TEMPERATURE), 1, 65_535)
            .map(|value| value as u16)
            .filter(|value| (TEMPERATURE_MIN..=TEMPERATURE_MAX).contains(value)),
        color: as_color(read_value(reply, PIID_COLOR)),
        mode: as_mode(read_value(reply, PIID_COLOR_MODE)),
    }
}

fn read_value(reply: &MiioReply, piid: u32) -> Option<&Value> {
    reply
        .result
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .find(|item| item.siid == SIID_LIGHT && item.piid == piid && item.code.unwrap_or(0) == 0)
        .and_then(|item| item.value.as_ref())
}

fn as_boolean(value: Option<&Value>) -> Option<bool> {
    match value {
        Some(Value::Bool(value)) => Some(*value),
        _ => None,
    }
}

fn as_integer(value: Option<&Value>, min: i64, max: i64) -> Option<i64> {
    let value = value?.as_i64()?;
    (min..=max).contains(&value).then_some(value)
}

fn as_color(value: Option<&Value>) -> Option<Rgb> {
    let value = value?.as_u64()?;
    Rgb::from_packed(u32::try_from(value).ok()?)
}

fn as_mode(value: Option<&Value>) -> ColorMode {
    match value.and_then(Value::as_i64) {
        Some(1) => ColorMode::Color,
        Some(2) => ColorMode::Temperature,
        _ => ColorMode::Unknown,
    }
}
