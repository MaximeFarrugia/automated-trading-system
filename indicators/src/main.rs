use anyhow::Context;
use diesel::{Connection, PgConnection};
use models::Candle;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    let redis_url = std::env::var("REDIS_URL").context("REDIS_URL fron .env file")?;
    let redis_client = redis::Client::open(redis_url)?;
    let mut redis_conn = redis_client.get_connection()?;
    let mut pubsub = redis_conn.as_pubsub();
    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL from .env file")?;
    let mut conn = PgConnection::establish(&database_url)?;

    pubsub.subscribe("candle_close")?;
    loop {
        let msg = pubsub.get_message()?;
        let payload: String = msg.get_payload()?;
        let data: Candle = serde_json::from_str(&payload)?;
    }
}
