use core::fmt;
use std::{borrow::Cow, ops::Deref};

use derive_builder::Builder;
use serde::{Deserialize, Serialize};

use crate::rest::{endpoint::Endpoint, params::QueryParams};

#[derive(Debug, Clone, Builder)]
pub struct Candles<'a> {
    product_id: Cow<'a, str>,
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
    granularity: Granularity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Granularity {
    #[serde(rename = "UNKNOWN_GRANULARITY")]
    Unknown,
    OneMinute,
    FiveMinute,
    FifteenMinute,
    ThirtyMinute,
    OneHour,
    TwoHour,
    SixHour,
    OneDay,
}

impl fmt::Display for Granularity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Granularity::Unknown => "UNKNOWN_GRANULARITY",
            Granularity::OneMinute => "ONE_MINUTE",
            Granularity::FiveMinute => "FIVE_MINUTE",
            Granularity::FifteenMinute => "FIFTEEN_MINUTE",
            Granularity::ThirtyMinute => "THIRTY_MINUTE",
            Granularity::OneHour => "ONE_HOUR",
            Granularity::TwoHour => "TWO_HOUR",
            Granularity::SixHour => "SIX_HOUR",
            Granularity::OneDay => "ONE_DAY",
        };
        return write!(f, "{value}");
    }
}

impl<'a> Endpoint for Candles<'a> {
    fn endpoint(&self) -> Cow<'static, str> {
        return format!(
            "api/v3/brokerage/products/{}/candles",
            self.product_id.deref()
        )
        .into();
    }

    fn params(&self) -> QueryParams {
        let mut params = QueryParams::default();

        params
            .push("start", self.start.timestamp().to_string())
            .push("end", self.end.timestamp().to_string())
            .push("granularity", self.granularity.to_string());
        return params;
    }
}
