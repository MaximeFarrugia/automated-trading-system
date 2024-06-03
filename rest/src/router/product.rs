use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use chrono::TimeZone;
use diesel::prelude::*;
use models::{schema::candles, Candle};

use crate::{error::AppError, AppState};

use super::Pagination;

pub fn create_router() -> Router<AppState> {
    let router = Router::new().route("/:product_id/candles", get(get_candles));

    return router;
}

async fn get_candles(
    State(state): State<AppState>,
    Path(product_id): Path<String>,
    Query(params): Query<Pagination>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pg_conn = &mut state.pg_pool.get()?;
    let start_timestamp = chrono::Utc
        .timestamp_opt(i64::from(params.start_timestamp), 0)
        .unwrap();
    let end_timestamp = chrono::Utc
        .timestamp_opt(i64::from(params.end_timestamp), 0)
        .unwrap();
    let res = candles::table
        .select(Candle::as_select())
        .filter(
            candles::pair
                .eq(&product_id)
                .and(candles::timeframe.eq(params.timeframe.to_string()))
                .and(candles::open_time.ge(start_timestamp))
                .and(candles::open_time.le(end_timestamp)),
        )
        .order(candles::open_time.asc())
        .get_results(pg_conn)?;
    return Ok(Json(serde_json::json!(res)));
}
