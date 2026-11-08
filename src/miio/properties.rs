use serde::Serialize;
use serde_json::Value;

use crate::bulb::model::SIID_LIGHT;
use crate::error::MiioError;
use crate::miio::reply::MiioReply;

#[derive(Clone, Debug, Serialize)]
pub struct PropertyParam {
    pub did: String,
    pub siid: u32,
    pub piid: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
}

pub fn properties(piids: &[u32]) -> Vec<PropertyParam> {
    piids
        .iter()
        .map(|piid| PropertyParam {
            did: format!("{SIID_LIGHT}-{piid}"),
            siid: SIID_LIGHT,
            piid: *piid,
            value: None,
        })
        .collect()
}

pub fn properties_with_value(piid: u32, value: Value) -> Vec<PropertyParam> {
    vec![PropertyParam {
        did: format!("{SIID_LIGHT}-{piid}"),
        siid: SIID_LIGHT,
        piid,
        value: Some(value),
    }]
}

pub fn reply_error(reply: &MiioReply, address: &str) -> Option<MiioError> {
    reply.error.as_ref().map(|error| MiioError::Device {
        address: address.to_owned(),
        message: format!("{} ({})", error.message, error.code),
    })
}

pub fn property_error(reply: &MiioReply, address: &str) -> Option<MiioError> {
    for item in reply.result.as_deref().unwrap_or(&[]) {
        if let Some(code) = item.code
            && code != 0
        {
            return Some(MiioError::Device {
                address: address.to_owned(),
                message: format!(
                    "Property {}-{} failed with code {code}.",
                    item.siid, item.piid
                ),
            });
        }
    }
    None
}

pub fn command_error(reply: &MiioReply, address: &str) -> Option<MiioError> {
    reply_error(reply, address).or_else(|| property_error(reply, address))
}
