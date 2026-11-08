use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, Deserialize)]
pub struct MiioReply {
    #[serde(default)]
    pub id: Option<u32>,
    #[serde(default)]
    pub result: Option<Vec<MiioResultItem>>,
    #[serde(default)]
    pub error: Option<MiioErrorPayload>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MiioResultItem {
    pub siid: u32,
    pub piid: u32,
    #[serde(default)]
    pub value: Option<Value>,
    #[serde(default)]
    pub code: Option<i64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MiioErrorPayload {
    pub code: i64,
    pub message: String,
}

pub fn decode_reply(body: &[u8]) -> Result<MiioReply, String> {
    serde_json::from_slice(body)
        .map_err(|error| format!("The bulb sent a reply jyoti could not read: {error}"))
}
