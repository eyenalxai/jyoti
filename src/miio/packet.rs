use aes::Aes128;
use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{BlockModeDecrypt, BlockModeEncrypt, KeyIvInit};
use md5::{Digest, Md5};
use serde_json::Value;

use crate::bulb::model::token_bytes;

type Aes128CbcEnc = cbc::Encryptor<Aes128>;
type Aes128CbcDec = cbc::Decryptor<Aes128>;

const HELLO: [u8; 32] = [
    0x21, 0x31, 0x00, 0x20, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
];
const MAGIC_FIRST: u8 = 0x21;
const MAGIC_SECOND: u8 = 0x31;
const HEADER_SIZE: usize = 16;
const CHECKSUM_SIZE: usize = 16;
const PACKET_PREFIX: usize = HEADER_SIZE + CHECKSUM_SIZE;

#[derive(Clone, Debug)]
pub struct HelloReply {
    pub device_id: [u8; 4],
    pub stamp: [u8; 4],
}

#[derive(Clone, Debug)]
pub struct ParsedPacket {
    pub device_id: [u8; 4],
    pub stamp: [u8; 4],
    pub body: Vec<u8>,
}

pub fn build_hello() -> [u8; 32] {
    HELLO
}

pub fn build_request(
    token: &str,
    device_id: [u8; 4],
    stamp: [u8; 4],
    message_id: u32,
    method: &str,
    params: &Value,
) -> Result<Vec<u8>, String> {
    let body = serde_json::to_vec(&serde_json::json!({
        "id": message_id,
        "method": method,
        "params": params,
    }))
    .map_err(|error| error.to_string())?;
    build_packet(token, device_id, stamp, &body)
}

pub fn build_packet(
    token: &str,
    device_id: [u8; 4],
    stamp: [u8; 4],
    body: &[u8],
) -> Result<Vec<u8>, String> {
    let secret = token_bytes(token)
        .ok_or_else(|| "The token must be 32 hexadecimal characters.".to_owned())?;
    let encrypted = encrypt(&secret, body)?;
    let total = PACKET_PREFIX + encrypted.len();
    if total > u16::MAX as usize {
        return Err("The miIO packet is too large.".to_owned());
    }

    let mut header = [0u8; HEADER_SIZE];
    let prefix = [
        MAGIC_FIRST,
        MAGIC_SECOND,
        (total / 256) as u8,
        (total % 256) as u8,
    ];
    if let Some(slot) = header.get_mut(0..4) {
        slot.copy_from_slice(&prefix);
    }
    if let Some(slot) = header.get_mut(8..12) {
        slot.copy_from_slice(&device_id);
    }
    if let Some(slot) = header.get_mut(12..16) {
        slot.copy_from_slice(&stamp);
    }

    let mut checksum_hasher = Md5::new();
    checksum_hasher.update(header);
    checksum_hasher.update(secret);
    checksum_hasher.update(&encrypted);
    let checksum = checksum_hasher.finalize();

    let mut packet = Vec::with_capacity(total);
    packet.extend_from_slice(&header);
    packet.extend_from_slice(&checksum);
    packet.extend_from_slice(&encrypted);
    Ok(packet)
}

pub fn parse_hello_reply(packet: &[u8]) -> Result<HelloReply, String> {
    if packet.len() < PACKET_PREFIX {
        return Err("The bulb sent a short hello reply.".to_owned());
    }
    if packet.first().copied() != Some(MAGIC_FIRST) || packet.get(1).copied() != Some(MAGIC_SECOND)
    {
        return Err("The bulb sent an unknown packet.".to_owned());
    }
    let length = read_u16(packet, 2)?;
    if length < PACKET_PREFIX || length > packet.len() {
        return Err("The bulb sent a malformed hello reply.".to_owned());
    }
    Ok(HelloReply {
        device_id: packet
            .get(8..12)
            .and_then(|slice| slice.try_into().ok())
            .ok_or_else(|| "short device id".to_owned())?,
        stamp: packet
            .get(12..16)
            .and_then(|slice| slice.try_into().ok())
            .ok_or_else(|| "short stamp".to_owned())?,
    })
}

pub fn parse_packet(token: &str, packet: &[u8]) -> Result<ParsedPacket, String> {
    if packet.len() < PACKET_PREFIX {
        return Err("The bulb sent a short reply.".to_owned());
    }
    if packet.first().copied() != Some(MAGIC_FIRST) || packet.get(1).copied() != Some(MAGIC_SECOND)
    {
        return Err("The bulb sent an unknown packet.".to_owned());
    }
    let length = read_u16(packet, 2)?;
    if length < PACKET_PREFIX || length > packet.len() {
        return Err("The bulb sent a malformed reply.".to_owned());
    }

    let secret = token_bytes(token)
        .ok_or_else(|| "The token must be 32 hexadecimal characters.".to_owned())?;
    let header = packet
        .get(0..HEADER_SIZE)
        .ok_or_else(|| "The bulb sent a short reply.".to_owned())?;
    let checksum = packet
        .get(HEADER_SIZE..PACKET_PREFIX)
        .ok_or_else(|| "The bulb sent a short reply.".to_owned())?;
    let encrypted = packet
        .get(PACKET_PREFIX..length)
        .ok_or_else(|| "The bulb sent a short reply.".to_owned())?;

    let mut hasher = Md5::new();
    hasher.update(header);
    hasher.update(secret);
    hasher.update(encrypted);
    let expected = hasher.finalize();
    if checksum != expected.as_slice() {
        return Err("The bulb reply failed its checksum.".to_owned());
    }

    Ok(ParsedPacket {
        device_id: header
            .get(8..12)
            .and_then(|slice| slice.try_into().ok())
            .ok_or_else(|| "short device id".to_owned())?,
        stamp: header
            .get(12..16)
            .and_then(|slice| slice.try_into().ok())
            .ok_or_else(|| "short stamp".to_owned())?,
        body: decrypt(&secret, encrypted)?,
    })
}

fn read_u16(packet: &[u8], offset: usize) -> Result<usize, String> {
    let high = *packet
        .get(offset)
        .ok_or_else(|| "The bulb sent a short packet.".to_owned())?;
    let low = *packet
        .get(offset + 1)
        .ok_or_else(|| "The bulb sent a short packet.".to_owned())?;
    Ok(usize::from(high) * 256 + usize::from(low))
}

fn encrypt(secret: &[u8; 16], plain: &[u8]) -> Result<Vec<u8>, String> {
    let key = md5_bytes(secret, &[]);
    let iv = md5_bytes(&key, secret);
    let encrypted = Aes128CbcEnc::new(&key.into(), &iv.into()).encrypt_padded_vec::<Pkcs7>(plain);
    Ok(encrypted)
}

fn decrypt(secret: &[u8; 16], encrypted: &[u8]) -> Result<Vec<u8>, String> {
    let key = md5_bytes(secret, &[]);
    let iv = md5_bytes(&key, secret);
    Aes128CbcDec::new(&key.into(), &iv.into())
        .decrypt_padded_vec::<Pkcs7>(encrypted)
        .map_err(|_| "The bulb sent malformed encrypted data.".to_owned())
}

fn md5_bytes(first: &[u8], second: &[u8]) -> [u8; 16] {
    let mut hasher = Md5::new();
    hasher.update(first);
    hasher.update(second);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn a_packet_round_trips_through_encryption_and_checksum() -> Result<(), String> {
        let device_id = [1, 2, 3, 4];
        let stamp = [5, 6, 7, 8];
        let body = br#"{"id":1,"method":"get_properties","params":[]}"#;
        let packet = build_packet(TOKEN, device_id, stamp, body)?;
        let parsed = parse_packet(TOKEN, &packet)?;
        assert_eq!(parsed.device_id, device_id);
        assert_eq!(parsed.stamp, stamp);
        assert_eq!(parsed.body, body);
        Ok(())
    }

    #[test]
    fn a_hello_reply_reads_the_device_and_stamp() -> Result<(), String> {
        let reply = parse_hello_reply(&build_hello())?;
        assert_eq!(reply.device_id, [0xff; 4]);
        assert_eq!(reply.stamp, [0xff; 4]);
        Ok(())
    }

    #[test]
    fn the_wrong_token_fails_the_checksum() -> Result<(), String> {
        let packet = build_packet(TOKEN, [1, 2, 3, 4], [5, 6, 7, 8], b"{}")?;
        let wrong = "fedcba9876543210fedcba9876543210";
        assert!(parse_packet(wrong, &packet).is_err());
        Ok(())
    }
}
