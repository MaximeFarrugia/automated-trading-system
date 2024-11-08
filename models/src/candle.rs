use derive_builder::Builder;
use derive_getters::Getters;
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Builder, Getters, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::candles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Candle {
    pair: String,
    open_time: chrono::DateTime<chrono::Utc>,
    timeframe: String,
    open: Decimal,
    high: Decimal,
    low: Decimal,
    close: Decimal,
    size_in_millis: i64,
}
