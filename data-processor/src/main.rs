use anyhow::{anyhow, Context};
use coinbase_advanced_api::ws::channel::{ticker::TickerEvent, Response};
use diesel::{prelude::*, upsert::excluded};
use models::{
    candle::CandleBuilder,
    schema::candles::{self},
    Candle,
};
use types::Timeframe;

fn handle_ticker(
    payload: String,
    redis_conn: &mut redis::Connection,
    pg_conn: &mut PgConnection,
) -> anyhow::Result<()> {
    let data: Response<TickerEvent> = serde_json::from_str(&payload)?;
    let timeframes = [
        Timeframe::Minute(1),
        Timeframe::Minute(5),
        Timeframe::Minute(15),
        Timeframe::Hour(1),
        Timeframe::Hour(12),
        Timeframe::Day(1),
        Timeframe::Week(1),
        Timeframe::Month(1),
    ];

    for event in data.events() {
        for timeframe in timeframes.iter() {
            let candles: Vec<Candle> = event
                .tickers()
                .iter()
                .map(|ticker| {
                    let (open_time, size_in_millis) =
                        timeframe.open_and_size(data.timestamp()).unwrap();
                    CandleBuilder::default()
                        .pair(ticker.product_id().to_owned())
                        .open(ticker.price().to_owned())
                        .high(ticker.price().to_owned())
                        .low(ticker.price().to_owned())
                        .close(ticker.price().to_owned())
                        .open_time(open_time)
                        .timeframe(timeframe.to_string())
                        .size_in_millis(size_in_millis)
                        .build()
                        .unwrap()
                })
                .collect();
            let result: Vec<Candle> = diesel::insert_into(candles::table)
                .values(candles)
                .on_conflict((candles::pair, candles::open_time, candles::timeframe))
                .do_update()
                .set((
                    candles::high.eq(diesel::dsl::sql("greatest(")
                        .bind(excluded(candles::high))
                        .sql(",")
                        .bind(candles::high)
                        .sql(")")),
                    candles::low.eq(diesel::dsl::sql("least(")
                        .bind(excluded(candles::low))
                        .sql(",")
                        .bind(candles::low)
                        .sql(")")),
                    candles::close.eq(excluded(candles::close)),
                ))
                .get_results(pg_conn)?;
            println!("{result:#?}");
        }
    }
    return Ok(());
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    let redis_url = std::env::var("REDIS_URL").context("REDIS_URL from .env file")?;
    let redis_client = redis::Client::open(redis_url)?;
    let mut redis_sub_conn = redis_client.get_connection()?;
    let mut pubsub = redis_sub_conn.as_pubsub();
    let mut redis_pub_conn = redis_client.get_connection()?;
    let database_url = std::env::var("DATABASE_URL").context("BATABASE_URL from .env file")?;
    let mut pg_conn = PgConnection::establish(&database_url)?;

    pubsub.subscribe("ticker")?;
    loop {
        let msg = pubsub.get_message()?;
        let payload: String = msg.get_payload()?;

        if let Err(err) = match msg
            .get_channel::<String>()
            .context("Getting channel from redis pubsub message")
        {
            Ok(channel) if channel == "ticker" => {
                handle_ticker(payload, &mut redis_pub_conn, &mut pg_conn)
            }
            Ok(channel) => Err(anyhow!("No handler for redis channel {channel}")),
            Err(err) => Err(err),
        } {
            println!("{err:#}");
        }
    }
}
