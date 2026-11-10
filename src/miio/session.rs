use std::io::ErrorKind;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::bulb::model::{
    BRIGHTNESS_MAX, BRIGHTNESS_MIN, BulbState, PIID_BRIGHTNESS, PIID_COLOR, PIID_POWER,
    PIID_TEMPERATURE, Rgb, TEMPERATURE_MAX, TEMPERATURE_MIN, token_bytes,
};
use crate::error::MiioError;
use crate::miio::packet::{build_hello, build_request, parse_hello_reply, parse_packet};
use crate::miio::properties::{PropertyParam, command_error, properties_with_value};
use crate::miio::reply::{MiioReply, decode_reply};
use crate::miio::state::{state_from_reply, state_properties};

const HELLO_TIMEOUT: Duration = Duration::from_secs(1);
const STREAM_TIMEOUT: Duration = Duration::from_millis(600);
const REHELLO_AFTER: Duration = Duration::from_secs(120);
const CONFIRMED_ATTEMPTS: usize = 3;
const MAX_MESSAGE_ID: u32 = 9_999;

pub struct MiioSession {
    socket: UdpSocket,
    target: SocketAddr,
    address: String,
    token: String,
    device_id: Option<[u8; 4]>,
    stamp: Option<[u8; 4]>,
    stamp_at: Option<Instant>,
    last_packet_at: Option<Instant>,
    message_id: u32,
    hello_timeout: Duration,
    stream_timeout: Duration,
}

impl MiioSession {
    pub fn open(
        address: IpAddr,
        token: &str,
        port: u16,
        timeout: Duration,
    ) -> Result<Self, MiioError> {
        let target = SocketAddr::new(address, port);
        let bind_address = if address.is_ipv6() {
            "[::]:0"
        } else {
            "0.0.0.0:0"
        };
        let address = address.to_string();
        token_bytes(token).ok_or_else(|| MiioError::Token {
            address: address.clone(),
        })?;
        let socket = UdpSocket::bind(bind_address).map_err(|error| MiioError::Network {
            address: address.to_owned(),
            message: error.to_string(),
        })?;
        socket
            .set_read_timeout(Some(STREAM_TIMEOUT))
            .map_err(|error| MiioError::Network {
                address: address.to_owned(),
                message: error.to_string(),
            })?;

        Ok(Self {
            socket,
            target,
            address: address.to_owned(),
            token: token.to_owned(),
            device_id: None,
            stamp: None,
            stamp_at: None,
            last_packet_at: None,
            message_id: 0,
            hello_timeout: HELLO_TIMEOUT.min(timeout),
            stream_timeout: STREAM_TIMEOUT.min(timeout),
        })
    }

    pub fn hello(&mut self) -> Result<(), MiioError> {
        self.send(&build_hello())?;
        let deadline = Instant::now() + self.hello_timeout;
        let packet = self.receive(deadline)?;
        let reply = parse_hello_reply(&packet).map_err(|message| MiioError::Protocol {
            address: self.address.clone(),
            message,
        })?;
        let now = Instant::now();
        self.device_id = Some(reply.device_id);
        self.stamp = Some(reply.stamp);
        self.stamp_at = Some(now);
        self.last_packet_at = Some(now);
        Ok(())
    }

    pub fn hello_if_stale(&mut self) -> Result<(), MiioError> {
        let stale = self
            .last_packet_at
            .is_none_or(|at| at.elapsed() > REHELLO_AFTER);
        if stale || self.device_id.is_none() || self.stamp.is_none() {
            self.hello()?;
        }
        Ok(())
    }

    pub fn get_state(&mut self) -> Result<BulbState, MiioError> {
        let params = state_properties();
        let reply = self.request(
            "get_properties",
            &serde_json::to_value(params).map_err(|error| MiioError::Protocol {
                address: self.address.clone(),
                message: error.to_string(),
            })?,
        )?;
        if let Some(error) = command_error(&reply, &self.address) {
            return Err(error);
        }
        Ok(state_from_reply(&reply))
    }

    pub fn set_power(&mut self, on: bool) -> Result<(), MiioError> {
        self.set_properties(properties_with_value(PIID_POWER, json!(on)))
    }

    pub fn set_brightness(&mut self, value: u8) -> Result<(), MiioError> {
        if !(BRIGHTNESS_MIN..=BRIGHTNESS_MAX).contains(&value) {
            return Err(MiioError::Value {
                address: self.address.clone(),
                message: format!("Brightness must be {BRIGHTNESS_MIN}–{BRIGHTNESS_MAX}."),
            });
        }
        self.set_properties(properties_with_value(PIID_BRIGHTNESS, json!(value)))
    }

    pub fn set_temperature(&mut self, kelvin: u16) -> Result<(), MiioError> {
        if !(TEMPERATURE_MIN..=TEMPERATURE_MAX).contains(&kelvin) {
            return Err(MiioError::Value {
                address: self.address.clone(),
                message: format!(
                    "Color temperature must be {TEMPERATURE_MIN}–{TEMPERATURE_MAX} K."
                ),
            });
        }
        self.set_properties(properties_with_value(PIID_TEMPERATURE, json!(kelvin)))
    }

    pub fn set_color(&mut self, color: Rgb) -> Result<(), MiioError> {
        self.set_properties(properties_with_value(PIID_COLOR, json!(color.pack())))
    }

    pub fn set_properties(&mut self, params: Vec<PropertyParam>) -> Result<(), MiioError> {
        let value = serde_json::to_value(params).map_err(|error| MiioError::Protocol {
            address: self.address.clone(),
            message: error.to_string(),
        })?;
        let reply = self.request("set_properties", &value)?;
        if let Some(error) = command_error(&reply, &self.address) {
            return Err(error);
        }
        Ok(())
    }

    pub fn confirm_properties_until(
        &mut self,
        params: Vec<PropertyParam>,
        deadline: Instant,
    ) -> Result<(), MiioError> {
        let mut last = MiioError::Timeout {
            address: self.address.clone(),
        };
        for _ in 0..CONFIRMED_ATTEMPTS {
            if Instant::now() >= deadline {
                break;
            }
            match self.set_properties(params.clone()) {
                Ok(()) => return Ok(()),
                Err(error) if error.is_device() => return Err(error),
                Err(error) => last = error,
            }
        }
        Err(last)
    }

    fn request(&mut self, method: &str, params: &Value) -> Result<MiioReply, MiioError> {
        self.hello_if_stale()?;
        match self.exchange(method, params) {
            Ok(reply) => Ok(reply),
            Err(MiioError::Protocol { .. }) => {
                self.hello()?;
                self.exchange(method, params)
            }
            Err(error) => Err(error),
        }
    }

    fn exchange(&mut self, method: &str, params: &Value) -> Result<MiioReply, MiioError> {
        let device_id = self.device_id.ok_or_else(|| MiioError::Protocol {
            address: self.address.clone(),
            message: "The session has no device ID. Reconnect and try again.".to_owned(),
        })?;
        let stamp = self.advanced_stamp().ok_or_else(|| MiioError::Protocol {
            address: self.address.clone(),
            message: "The session has no stamp. Reconnect and try again.".to_owned(),
        })?;
        let message_id = self.next_message_id();
        let packet = build_request(&self.token, device_id, stamp, message_id, method, params)
            .map_err(|message| MiioError::Protocol {
                address: self.address.clone(),
                message,
            })?;
        self.send(&packet)?;

        let deadline = Instant::now() + self.stream_timeout;
        loop {
            let packet = self.receive(deadline)?;
            let parsed =
                parse_packet(&self.token, &packet).map_err(|message| MiioError::Protocol {
                    address: self.address.clone(),
                    message,
                })?;
            let reply = decode_reply(&parsed.body).map_err(|message| MiioError::Protocol {
                address: self.address.clone(),
                message,
            })?;
            let now = Instant::now();
            self.device_id = Some(parsed.device_id);
            self.stamp = Some(parsed.stamp);
            self.stamp_at = Some(now);
            self.last_packet_at = Some(now);
            if reply.id == Some(message_id) {
                return Ok(reply);
            }
        }
    }

    fn send(&self, packet: &[u8]) -> Result<(), MiioError> {
        self.socket
            .send_to(packet, self.target)
            .map(|_| ())
            .map_err(|error| MiioError::Network {
                address: self.address.clone(),
                message: error.to_string(),
            })
    }

    fn receive(&self, deadline: Instant) -> Result<Vec<u8>, MiioError> {
        let mut buffer = [0u8; 4096];
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(timeout(&self.address));
            }
            self.socket
                .set_read_timeout(Some(remaining))
                .map_err(|error| MiioError::Network {
                    address: self.address.clone(),
                    message: error.to_string(),
                })?;
            match self.socket.recv_from(&mut buffer) {
                Ok((size, from)) if from.ip() == self.target.ip() => {
                    return buffer
                        .get(..size)
                        .map(<[u8]>::to_vec)
                        .ok_or_else(|| timeout(&self.address));
                }
                Ok(_) => continue,
                Err(error)
                    if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
                {
                    return Err(timeout(&self.address));
                }
                Err(error) => {
                    return Err(MiioError::Network {
                        address: self.address.clone(),
                        message: error.to_string(),
                    });
                }
            }
        }
    }

    fn next_message_id(&mut self) -> u32 {
        self.message_id = (self.message_id % MAX_MESSAGE_ID) + 1;
        self.message_id
    }

    fn advanced_stamp(&self) -> Option<[u8; 4]> {
        let stamp = self.stamp?;
        let stamp_at = self.stamp_at?;
        let elapsed = stamp_at.elapsed().as_secs() as u32;
        Some(
            u32::from_be_bytes(stamp)
                .wrapping_add(elapsed)
                .to_be_bytes(),
        )
    }
}

fn timeout(address: &str) -> MiioError {
    MiioError::Timeout {
        address: address.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv6Addr};
    use std::time::Duration;

    use super::MiioSession;
    use crate::error::MiioError;

    #[test]
    fn a_session_reports_the_parsed_address_on_a_bad_token() {
        let error = MiioSession::open(
            IpAddr::from(Ipv6Addr::LOCALHOST),
            "short",
            54_321,
            Duration::from_secs(1),
        );

        assert!(matches!(
            error,
            Err(MiioError::Token { address }) if address == "::1"
        ));
    }
}
