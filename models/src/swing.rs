use derive_builder::Builder;
use derive_getters::Getters;
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Builder, Getters, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::swings)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Swing {
    pair: String,
    open_time: chrono::DateTime<chrono::Utc>,
    timeframe: String,
    price: Decimal,
    flow: String,
    close_time: Option<chrono::DateTime<chrono::Utc>>,
}
