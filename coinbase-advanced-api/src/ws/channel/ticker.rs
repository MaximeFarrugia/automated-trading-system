use std::borrow::Cow;

use derive_builder::Builder;
use derive_getters::Getters;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::{Channel, EventType};

#[derive(Debug, Builder)]
pub struct TickerChannel<'a> {
    product_id: Cow<'a, str>,
}

#[derive(Debug, Getters, Serialize, Deserialize)]
pub struct TickerEvent {
    pub r#type: EventType,
    pub tickers: Vec<Ticker>,
}

#[derive(Debug, Getters, Serialize, Deserialize)]
pub struct Ticker {
    pub r#type: String,
    pub product_id: String,
    pub price: Decimal,
    pub volume_24_h: Decimal,
    pub low_24_h: Decimal,
    pub high_24_h: Decimal,
    pub low_52_w: Decimal,
    pub high_52_w: Decimal,
    pub price_percent_chg_24_h: Decimal,
    pub best_bid: Option<Decimal>,
    pub best_bid_quantity: Option<Decimal>,
    pub best_ask: Option<Decimal>,
    pub best_ask_quantity: Option<Decimal>,
}

impl<'a> Channel for TickerChannel<'a> {
    fn name(&self) -> Cow<'_, str> {
        return Cow::Borrowed("ticker");
    }

    fn product_id(&self) -> Cow<'_, str> {
        return self.product_id.clone();
    }
}
