use anyhow::Context;
use coinbase_advanced_api::ws::channel::{ticker::TickerEvent, Response};
use diesel::{prelude::*, upsert::excluded, PgConnection, r2d2::ConnectionManager};
use models::{candle::CandleBuilder, schema::candles, Candle};
use types::Timeframe;

pub fn handle_ticker(
    payload: String,
    redis_pool: r2d2::Pool<redis::Client>,
    pg_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
) -> anyhow::Result<()> {
    let pg_conn = &mut pg_pool.get().context("Getting connection from pg_pool")?;
    let data: Response<TickerEvent> = serde_json::from_str(&payload)?;
    let timeframes = [
        Timeframe::Minute(0),
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
