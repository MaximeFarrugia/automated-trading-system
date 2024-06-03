mod backtesting;
mod product;

use axum::Router;
use serde::Deserialize;
use tower_http::cors::{Any, CorsLayer};
use types::Timeframe;

use crate::AppState;

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new().allow_origin(Any);
    let router = Router::new()
        .nest("/backtesting", backtesting::create_router())
        .nest("/product", product::create_router())
        .with_state(state)
        .layer(cors);

    return router;
}

#[derive(Debug, Deserialize)]
struct Pagination {
    start_timestamp: u32,
    end_timestamp: u32,
    timeframe: Timeframe,
}
