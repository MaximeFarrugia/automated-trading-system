mod ticker;

use std::future;

use anyhow::{bail, Context};
use diesel::{PgConnection, r2d2::ConnectionManager};
use tracing::error;

async fn handle_redis_message(
    msg: redis::Msg,
    redis_pool: r2d2::Pool<redis::Client>,
    pg_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
) -> anyhow::Result<()> {
    let channel: String = msg
        .get_channel()
        .context("channel from redis pubsub message")?;
    let payload: String = msg.get_payload()?;

    match channel.as_str() {
        "ticker" => ticker::handle_ticker(payload, redis_pool, pg_pool)?,
        _ => bail!("No handler for redis channel {channel}"),
    };
    return Ok(());
}

fn init_pg_pool() -> anyhow::Result<r2d2::Pool<ConnectionManager<PgConnection>>> {
    let database_url = std::env::var("DATABASE_URL").context("BATABASE_URL from .env file")?;
    let manager = ConnectionManager::<PgConnection>::new(database_url);

    return r2d2::Pool::builder()
        .max_size(50)
        .min_idle(Some(10))
        .build(manager)
        .context("Creating PgPool");
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    tracing_subscriber::fmt::init();
    let pg_pool = init_pg_pool()?;
    let redis_pool = init_redis_pool()?;
    let mut redis_sub_conn = redis_pool.get().context("Get redis_sub_conn from redis_pool")?;
    let mut pubsub = redis_sub_conn.as_pubsub();

    pubsub.subscribe("ticker")?;
    loop {
        let msg = pubsub.get_message()?;
        let pg_pool = pg_pool.clone();
        let redis_pool = redis_pool.clone();

        tokio::spawn(async {
            let res = handle_redis_message(msg, redis_pool, pg_pool).await;

            if let Err(err) = res {
                error!("{err:#}");
            }
            future::ready(())
        });
    }
}
