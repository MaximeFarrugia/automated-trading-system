use derive_builder::Builder;
use derive_getters::Getters;
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Queryable, Selectable, Insertable, Builder, Getters, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::trades)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Trade {
    pair: String,
    open_time: chrono::DateTime<chrono::Utc>,
    timeframe: String,
    fill_time: Option<chrono::DateTime<chrono::Utc>>,
    quantity: Decimal,
    entry: Decimal,
    stop_loss: Decimal,
    take_profit: Decimal,
    flow: String,
    close_time: Option<chrono::DateTime<chrono::Utc>>,
    close: Option<Decimal>,
}
