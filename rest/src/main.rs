mod error;
mod router;

use std::{net::SocketAddr, sync::Arc};

use anyhow::{bail, Context};
use axum::extract::ws::Message;
use diesel::{r2d2::ConnectionManager, PgConnection};
use serde::{Deserialize, Serialize};
use tokio::sync::{
    broadcast::{self, Sender},
    Mutex,
};
use tracing::error;

fn init_pg_pool(is_backtest: bool) -> anyhow::Result<r2d2::Pool<ConnectionManager<PgConnection>>> {
    let database_url = match is_backtest {
        false => std::env::var("DATABASE_URL").context("DATABASE_URL from .env file")?,
        true => std::env::var("BACKTEST_DATABASE_URL")
            .context("BACKTEST_DATABASE_URL from .env file")?,
    };
    let manager = ConnectionManager::<PgConnection>::new(database_url);

    return r2d2::Pool::builder()
        .max_size(50)
        .min_idle(Some(10))
        .build(manager)
        .context(format!("Creating PgPool is_backtest={is_backtest}"));
}

fn init_redis_pool() -> anyhow::Result<r2d2::Pool<redis::Client>> {
    let redis_url = std::env::var("REDIS_URL").context("REDIS_URL from .env file")?;
    let redis_client = redis::Client::open(redis_url)?;

    return r2d2::Pool::builder()
        .max_size(50)
        .min_idle(Some(10))
        .build(redis_client)
        .context("Creating RedisPool");
}

#[derive(Clone)]
pub struct AppState {
    pg_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    pg_pool_backtest: r2d2::Pool<ConnectionManager<PgConnection>>,
    redis_pool: r2d2::Pool<redis::Client>,
    broadcast_tx: Arc<Mutex<Sender<Message>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WsBroadcastMessage {
    Candle(String),
    BacktestCandle(String),
    Fvg(String),
    BacktestFvg(String),
}

async fn handle_redis_message(msg: redis::Msg, state: AppState) -> anyhow::Result<()> {
    let channel: String = msg
        .get_channel()
        .context("channel from redis pubsub message")?;
    let payload: String = msg.get_payload()?;
    let msg = match channel.as_str() {
        "candle" => serde_json::to_string(&WsBroadcastMessage::Candle(payload))?,
        "backtest-candle_close" => serde_json::to_string(&WsBroadcastMessage::BacktestCandle(payload))?,
        "fvg" => serde_json::to_string(&WsBroadcastMessage::Fvg(payload))?,
        "backtest-fvg" => serde_json::to_string(&WsBroadcastMessage::BacktestFvg(payload))?,
        _ => bail!("No handler for redis channel {channel}"),
    };

    state
        .broadcast_tx
        .lock()
        .await
        .send(Message::Text(msg))
        .context("Sending redis msg on broadcast_tx")?;
    return Ok(());
}

async fn handle_redis_sub(state: AppState) -> anyhow::Result<()> {
    let mut redis_sub_conn = state
        .redis_pool
        .get()
        .context("Get redis_sub_conn from redis_pool")?;
    let mut pubsub = redis_sub_conn.as_pubsub();

    pubsub.subscribe("candle_close")?;
    pubsub.subscribe("backtest-candle_close")?;
    loop {
        let msg = pubsub.get_message()?;
        let res = handle_redis_message(msg, state.clone()).await;
        if let Err(err) = res {
            error!("{err:#}");
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    tracing_subscriber::fmt::init();
    let pg_pool = init_pg_pool(false)?;
    let pg_pool_backtest = init_pg_pool(true)?;
    let redis_pool = init_redis_pool()?;
    let (broadcast_tx, _broadcast_rx) = broadcast::channel(200);

    let state = AppState {
        pg_pool,
        pg_pool_backtest,
        redis_pool,
        broadcast_tx: Arc::new(Mutex::new(broadcast_tx)),
    };

    let state_tmp = state.clone();
    tokio::spawn(async {
        if let Err(err) = handle_redis_sub(state_tmp).await {
            tracing::error!("{err:#}");
        }
    });

    let addr = SocketAddr::from(([127, 0, 0, 1], 3003));
    tracing::debug!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let app = router::create_router(state);
    axum::serve(listener, app).await.unwrap();
    return Ok(());
}
