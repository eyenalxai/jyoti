use std::net::IpAddr;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub const BRIGHTNESS_MIN: u8 = 1;
pub const BRIGHTNESS_MAX: u8 = 100;
pub const TEMPERATURE_MIN: u16 = 2_700;
pub const TEMPERATURE_MAX: u16 = 6_500;
pub const NEUTRAL_TEMPERATURE: u16 = 4_000;

pub const SIID_LIGHT: u32 = 2;
pub const PIID_POWER: u32 = 1;
pub const PIID_BRIGHTNESS: u32 = 2;
pub const PIID_TEMPERATURE: u32 = 3;
pub const PIID_COLOR: u32 = 4;
pub const PIID_COLOR_MODE: u32 = 6;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Bulb {
    pub id: String,
    pub name: String,
    pub address: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BulbFile {
    pub version: u8,
    pub bulbs: Vec<Bulb>,
}

impl BulbFile {
    pub const VERSION: u8 = 1;
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BulbState {
    pub power: Option<bool>,
    pub brightness: Option<u8>,
    pub temperature: Option<u16>,
    pub color: Option<Rgb>,
    pub mode: ColorMode,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorMode {
    Color,
    Temperature,
    #[default]
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Rgb {
    pub const WHITE: Rgb = Rgb {
        red: 255,
        green: 255,
        blue: 255,
    };
    pub fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    pub fn pack(self) -> u32 {
        u32::from(self.red) * 65_536 + u32::from(self.green) * 256 + u32::from(self.blue)
    }

    pub fn from_packed(value: u32) -> Option<Self> {
        (value <= 16_777_215).then_some(Self {
            red: ((value >> 16) & 0xff) as u8,
            green: ((value >> 8) & 0xff) as u8,
            blue: (value & 0xff) as u8,
        })
    }

    pub fn hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.red, self.green, self.blue)
    }

    pub fn parse_hex(value: &str) -> Option<Self> {
        let value = value.trim().trim_start_matches('#');
        let digits: [u8; 6] = value.as_bytes().try_into().ok()?;
        Some(Self {
            red: hex_byte(digits[0], digits[1])?,
            green: hex_byte(digits[2], digits[3])?,
            blue: hex_byte(digits[4], digits[5])?,
        })
    }
}

fn hex_byte(high: u8, low: u8) -> Option<u8> {
    let digits = [high, low];
    let text = std::str::from_utf8(&digits).ok()?;
    u8::from_str_radix(text, 16).ok()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BulbField {
    Name,
    Address,
    Model,
    Token,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldIssue {
    pub field: BulbField,
    pub message: String,
}

impl FieldIssue {
    pub fn new(field: BulbField, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }
}

pub fn is_token(value: &str) -> bool {
    value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub fn token_bytes(value: &str) -> Option<[u8; 16]> {
    if !is_token(value) {
        return None;
    }
    let mut bytes = [0u8; 16];
    for (index, slot) in bytes.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(bytes)
}

pub fn validate_draft(
    name: &str,
    address: &str,
    model: &str,
    token: &str,
    token_required: bool,
) -> Vec<FieldIssue> {
    let mut issues = Vec::new();
    let name = name.trim();
    let address = address.trim();
    let model = model.trim();
    let token = token.trim();

    if name.is_empty() {
        issues.push(FieldIssue::new(BulbField::Name, "Give the bulb a name."));
    } else if name.chars().count() > 60 {
        issues.push(FieldIssue::new(
            BulbField::Name,
            "Keep the name under 60 characters.",
        ));
    }

    if address.is_empty() {
        issues.push(FieldIssue::new(
            BulbField::Address,
            "Enter the bulb's IP address.",
        ));
    } else if IpAddr::from_str(address).is_err() {
        issues.push(FieldIssue::new(
            BulbField::Address,
            "Use an IPv4 or IPv6 address, for example 192.168.1.42.",
        ));
    }

    if model.chars().count() > 60 {
        issues.push(FieldIssue::new(
            BulbField::Model,
            "Keep the model under 60 characters.",
        ));
    }

    if token.is_empty() {
        if token_required {
            issues.push(FieldIssue::new(
                BulbField::Token,
                "Paste the 32-character token from the Xiaomi app or setup script.",
            ));
        }
    } else if !is_token(token) {
        issues.push(FieldIssue::new(
            BulbField::Token,
            "A token is 32 hexadecimal characters.",
        ));
    }

    issues
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn trim_model(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

pub fn brightness_from_fraction(fraction: f32) -> u8 {
    let value = (fraction.clamp(0.0, 1.0) * f32::from(BRIGHTNESS_MAX)).round();
    value.clamp(f32::from(BRIGHTNESS_MIN), f32::from(BRIGHTNESS_MAX)) as u8
}

pub fn temperature_from_fraction(fraction: f32) -> u16 {
    let span = f32::from(TEMPERATURE_MAX - TEMPERATURE_MIN);
    let value = f32::from(TEMPERATURE_MIN) + fraction.clamp(0.0, 1.0) * span;
    (value.round() as u16).clamp(TEMPERATURE_MIN, TEMPERATURE_MAX)
}

pub fn fraction_of_brightness(value: u8) -> f32 {
    (f32::from(value) - f32::from(BRIGHTNESS_MIN))
        / (f32::from(BRIGHTNESS_MAX) - f32::from(BRIGHTNESS_MIN))
}

pub fn fraction_of_temperature(value: u16) -> f32 {
    (f32::from(value) - f32::from(TEMPERATURE_MIN))
        / (f32::from(TEMPERATURE_MAX) - f32::from(TEMPERATURE_MIN))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_thirty_two_hex_characters() {
        assert!(is_token("0123456789abcdef0123456789abcdef"));
        assert!(!is_token("0123456789abcdef"));
        assert!(token_bytes("0123456789abcdef0123456789abcdef").is_some());
    }

    #[test]
    fn rgb_round_trips_through_hex() {
        let color = Rgb::new(255, 136, 0);
        assert_eq!(color.hex(), "#FF8800");
        assert_eq!(Rgb::parse_hex("#ff8800"), Some(color));
        assert_eq!(Rgb::parse_hex("ff8800"), Some(color));
        assert_eq!(Rgb::parse_hex("#ff88"), None);
        assert_eq!(Rgb::parse_hex("#ff880000"), None);
    }

    #[test]
    fn non_ascii_hex_is_rejected_without_panicking() {
        assert_eq!(Rgb::parse_hex("\u{20ac}\u{20ac}"), None);
        assert_eq!(Rgb::parse_hex("#\u{20ac}\u{20ac}"), None);
    }

    #[test]
    fn a_draft_checks_the_address_and_token() {
        let issues = validate_draft(
            "Desk lamp",
            "192.168.1.42",
            "",
            "0123456789abcdef0123456789abcdef",
            true,
        );
        assert!(issues.is_empty());

        let issues = validate_draft("", "not-an-ip", "", "short", true);
        assert!(issues.iter().any(|issue| issue.field == BulbField::Name));
        assert!(issues.iter().any(|issue| issue.field == BulbField::Address));
        assert!(issues.iter().any(|issue| issue.field == BulbField::Token));
    }
}
