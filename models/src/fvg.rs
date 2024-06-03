use derive_builder::Builder;
use derive_getters::Getters;
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Queryable, Selectable, Insertable, Builder, Getters, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::fvgs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FVG {
    pair: String,
    open_time: chrono::DateTime<chrono::Utc>,
    timeframe: String,
    high: Decimal,
    low: Decimal,
    flow: String,
    close_time: Option<chrono::DateTime<chrono::Utc>>,
}
