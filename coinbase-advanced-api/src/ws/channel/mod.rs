pub mod ticker;
pub mod ticker_batch;

use std::borrow::Cow;

use anyhow::Context;
use derive_getters::Getters;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::error::Error;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventType {
    Snapshot,
    Update,
}

#[derive(Debug, Getters, Serialize, Deserialize)]
pub struct Response<T> {
    channel: String,
    client_id: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    sequence_num: usize,
    events: Vec<T>,
}

pub trait Channel {
    fn name(&self) -> Cow<'_, str>;

    fn product_id(&self) -> Cow<'_, str>;

    fn parse<T: DeserializeOwned>(
        message: tokio_tungstenite::tungstenite::Message,
    ) -> anyhow::Result<T, Error> {
        let json_message = message.to_string();
        let res: T = serde_json::from_str(&json_message)
            .context(format!("Parsing json_message=[{json_message}]"))?;
        return Ok(res);
    }
}
