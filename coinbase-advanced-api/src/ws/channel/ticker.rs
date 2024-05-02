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
    r#type: EventType,
    tickers: Vec<Ticker>,
}

#[derive(Debug, Getters, Serialize, Deserialize)]
pub struct Ticker {
    r#type: String,
    product_id: String,
    price: Decimal,
    volume_24_h: Decimal,
    low_24_h: Decimal,
    high_24_h: Decimal,
    low_52_w: Decimal,
    high_52_w: Decimal,
    price_percent_chg_24_h: Decimal,
    best_bid: Option<Decimal>,
    best_bid_quantity: Option<Decimal>,
    best_ask: Option<Decimal>,
    best_ask_quantity: Option<Decimal>,
}

impl<'a> Channel for TickerChannel<'a> {
    fn name(&self) -> Cow<'_, str> {
        return Cow::Borrowed("ticker");
    }

    fn product_id(&self) -> Cow<'_, str> {
        return self.product_id.clone();
    }
}
