mod ticker;

use std::future;

use anyhow::{bail, Context};
use diesel::{r2d2::ConnectionManager, PgConnection};
use tracing::error;

async fn handle_redis_message(msg: redis::Msg, state: AppState) -> anyhow::Result<()> {
    let channel: String = msg
        .get_channel()
        .context("channel from redis pubsub message")?;
    let payload: String = msg.get_payload()?;

    match channel.as_str() {
        "ticker" => ticker::handle_ticker(payload, state.redis_pool, state.pg_pool, false)?,
        "backtest-ticker" => {
            ticker::handle_ticker(payload, state.redis_pool, state.pg_pool_backtest, true)?
        }
        _ => bail!("No handler for redis channel {channel}"),
    };
    return Ok(());
}

fn init_pg_pool(is_backtest: bool) -> anyhow::Result<r2d2::Pool<ConnectionManager<PgConnection>>> {
    let database_url = match is_backtest {
        false => std::env::var("DATABASE_URL").context("DATABASE_URL from .env file")?,
        true => std::env::var("BACKTEST_DATABASE_URL")
            .context("BACKTEST_DATABASE_URL from .env file")?,
    };
    let manager = ConnectionManager::<PgConnection>::new(database_url);

    return r2d2::Pool::builder()
        .max_size(5)
        .min_idle(Some(1))
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

pub struct AppState {
    pg_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    pg_pool_backtest: r2d2::Pool<ConnectionManager<PgConnection>>,
    redis_pool: r2d2::Pool<redis::Client>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    tracing_subscriber::fmt::init();
    let pg_pool = init_pg_pool(false)?;
    let pg_pool_backtest = init_pg_pool(true)?;
    let redis_pool = init_redis_pool()?;
    let mut redis_sub_conn = redis_pool
        .get()
        .context("Get redis_sub_conn from redis_pool")?;
    let mut pubsub = redis_sub_conn.as_pubsub();

    pubsub.subscribe("ticker")?;
    pubsub.subscribe("backtest-ticker")?;
    loop {
        let msg = pubsub.get_message()?;
        let pg_pool = pg_pool.clone();
        let pg_pool_backtest = pg_pool_backtest.clone();
        let redis_pool = redis_pool.clone();
        let state = AppState {
            pg_pool,
            pg_pool_backtest,
            redis_pool,
        };

        tokio::spawn(async {
            let res = handle_redis_message(msg, state).await;

            if let Err(err) = res {
                error!("{err:#}");
            }
            future::ready(())
        });
    }
}
