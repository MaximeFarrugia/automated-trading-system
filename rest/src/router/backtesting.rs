use std::{borrow::Cow, str::FromStr, sync::Arc};

use anyhow::Context;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, Query, State, WebSocketUpgrade,
    },
    response::Response as AxumResponse,
    routing::get,
    Json, Router,
};
use chrono::TimeZone;
use coinbase_advanced_api::{
    rest::{
        client::RestClient, products::candles::CandlesBuilder, query::Query as CoinbaseRestQuery,
    },
    ws::channel::{
        ticker::{Ticker, TickerEvent},
        EventType, Response,
    },
};
use diesel::prelude::*;
use futures::{stream::SplitSink, SinkExt, StreamExt};
use models::{
    fvg::FVG,
    schema::{candles, fvgs, trades, swings},
    Candle,
};
use redis::Commands;
use rust_decimal::Decimal;
use serde::Deserialize;
use tokio::sync::{broadcast::Receiver, Mutex};
use types::Timeframe;

use crate::{error::AppError, AppState, WsBroadcastMessage};

use super::Pagination;

pub fn create_router() -> Router<AppState> {
    let router = Router::new()
        .route("/:product_id/backtest", get(backtest))
        .route("/:product_id/candles", get(get_candles))
        .route("/:product_id/fvgs", get(get_fvgs))
        .route("/:product_id/ws", get(ws_handler));

    return router;
}

#[derive(Debug, Deserialize)]
struct CandlesResponse {
    candles: Vec<CoinbaseCandle>,
}

#[derive(Debug, Deserialize)]
struct CoinbaseCandle {
    start: String,
    open: String,
    high: String,
    low: String,
    close: String,
}

async fn backtest(
    State(state): State<AppState>,
    Path(product_id): Path<String>,
    Query(params): Query<Pagination>,
) -> Result<(), AppError> {
    let api_key = std::env::var("CB_API_KEY").context("CB_API_KEY from .env file")?;
    let private_key = std::env::var("CB_PRIVATE_KEY").context("CB_PRIVATE_KEY from .env file")?;
    let pg_conn = &mut state.pg_pool_backtest.get()?;
    let redis_conn = &mut state.redis_pool.get()?;
    let mut start_timestamp = chrono::Utc
        .timestamp_opt(i64::from(params.start_timestamp), 0)
        .unwrap();
    let end_timestamp = chrono::Utc
        .timestamp_opt(i64::from(params.end_timestamp), 0)
        .unwrap();
    let rest_client = RestClient::new(&api_key, &private_key)?;

    diesel::delete(candles::table).execute(pg_conn)?;
    diesel::delete(fvgs::table).execute(pg_conn)?;
    diesel::delete(trades::table).execute(pg_conn)?;
    diesel::delete(swings::table).execute(pg_conn)?;
    while start_timestamp < end_timestamp {
        let candles_request = CandlesBuilder::default()
            .product_id(Cow::Borrowed("BTC-USD"))
            .start(start_timestamp)
            .end(start_timestamp + chrono::Duration::minutes(300))
            .granularity(coinbase_advanced_api::rest::products::candles::Granularity::OneMinute)
            .build()?;
        let mut candles: CandlesResponse = candles_request.query(&rest_client).await?;
        candles.candles.sort_by_key(|x| x.start.clone());
        for candle in candles.candles.iter() {
            for price in [&candle.open, &candle.low, &candle.high, &candle.close].iter() {
                let wef = Response::<TickerEvent> {
                    channel: "ticker".to_owned(),
                    client_id: "".to_owned(),
                    timestamp: chrono::Utc
                        .timestamp_opt(i64::from_str(&candle.start)?, 0)
                        .unwrap(),
                    sequence_num: 0,
                    events: vec![TickerEvent {
                        r#type: EventType::Snapshot,
                        tickers: vec![Ticker {
                            r#type: "ticker".to_owned(),
                            product_id: product_id.clone(),
                            price: Decimal::from_str(price)?,
                            volume_24_h: Decimal::from(0),
                            low_24_h: Decimal::from(0),
                            high_24_h: Decimal::from(0),
                            low_52_w: Decimal::from(0),
                            high_52_w: Decimal::from(0),
                            price_percent_chg_24_h: Decimal::from(0),
                            best_bid: None,
                            best_bid_quantity: None,
                            best_ask: None,
                            best_ask_quantity: None,
                        }],
                    }],
                };
                let json_message = serde_json::to_string(&wef)?;
                let _: () = redis_conn
                    .publish("backtest-ticker".to_owned(), json_message)
                    .context("Publishing to redis backtes-ticker channel")?;
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        }
        start_timestamp += chrono::Duration::minutes(300);
    }
    return Ok(());
}

#[derive(Debug, Deserialize)]
struct WsPagination {
    timeframe: Option<Timeframe>,
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(product_id): Path<String>,
    Query(params): Query<WsPagination>,
) -> AxumResponse {
    return ws.on_upgrade(|websocket| handle_socket(websocket, state, product_id, params));
}

async fn handle_socket(
    ws: WebSocket,
    state: AppState,
    product_id: String,
    params: WsPagination,
) {
    let (ws_tx, _) = ws.split();
    let ws_tx = Arc::new(Mutex::new(ws_tx));

    {
        let broadcast_rx = state.broadcast_tx.lock().await.subscribe();
        tokio::spawn(async move {
            if let Err(err) = recv_broadcast(ws_tx, broadcast_rx, product_id, params).await {
                tracing::error!("{err:#}");
            }
        });
    }
}

async fn recv_broadcast(
    client_tx: Arc<Mutex<SplitSink<WebSocket, Message>>>,
    mut broadcast_rx: Receiver<Message>,
    product_id: String,
    params: WsPagination,
) -> anyhow::Result<()> {
    loop {
        let msg = broadcast_rx.recv().await?;
        let payload: WsBroadcastMessage = serde_json::from_str(msg.to_text().context("ws Msg to_text")?).context("ws Msg json_parse")?;
        let res = match payload {
            WsBroadcastMessage::BacktestCandle(x) => {
                Some(serde_json::from_str::<serde_json::Value>(&x).context("BacktestCandle json_parse")?)
            }
            WsBroadcastMessage::BacktestFvg(x) => {
                Some(serde_json::from_str::<serde_json::Value>(&x).context("BacktestFvg json_parse")?)
            }
            WsBroadcastMessage::BacktestFvgClose(x) => {
                Some(serde_json::from_str::<serde_json::Value>(&x).context("BacktestFvgClose json_parse")?)
            }
            WsBroadcastMessage::BacktestSwing(x) => {
                Some(serde_json::from_str::<serde_json::Value>(&x).context("BacktestSwing json_parse")?)
            }
            WsBroadcastMessage::BacktestSwingClose(x) => {
                Some(serde_json::from_str::<serde_json::Value>(&x).context("BacktestSwingClose json_parse")?)
            }
            WsBroadcastMessage::BacktestStrategyFvg(x) => {
                println!("wef");
                Some(serde_json::from_str::<serde_json::Value>(&x).context("BacktestStrategyFvg json_parse")?)
            }
            _ => None,
        };
        if let Some(x) = res {
            let pair = x.get("pair");
            let timeframe = x.get("timeframe");
            let pair_match = pair.is_some_and(|x| x.as_str().unwrap() == product_id.as_str());
            let timeframe_match = if let Some(tf) = params.timeframe.as_ref() {
                timeframe.is_some_and(|x| x.as_str().unwrap() == tf.to_string())
            } else {
                true
            };
            if pair_match && timeframe_match {
                client_tx.lock().await.send(msg).await.context("Send Msg to ws client")?;
            }
        }
    }
}

async fn get_candles(
    State(state): State<AppState>,
    Path(product_id): Path<String>,
    Query(params): Query<Pagination>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pg_conn = &mut state.pg_pool_backtest.get()?;
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

async fn get_fvgs(
    State(state): State<AppState>,
    Path(product_id): Path<String>,
    Query(params): Query<Pagination>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pg_conn = &mut state.pg_pool_backtest.get()?;
    let start_timestamp = chrono::Utc
        .timestamp_opt(i64::from(params.start_timestamp), 0)
        .unwrap();
    let end_timestamp = chrono::Utc
        .timestamp_opt(i64::from(params.end_timestamp), 0)
        .unwrap();
    let res = fvgs::table
        .select(FVG::as_select())
        .filter(
            fvgs::pair
                .eq(&product_id)
                .and(fvgs::timeframe.eq(params.timeframe.to_string()))
                .and(fvgs::open_time.ge(start_timestamp))
                .and(fvgs::open_time.le(end_timestamp)),
        )
        .order(fvgs::open_time.asc())
        .get_results(pg_conn)?;
    return Ok(Json(serde_json::json!(res)));
}
