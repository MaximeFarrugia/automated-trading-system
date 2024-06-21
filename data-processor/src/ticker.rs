use anyhow::Context;
use coinbase_advanced_api::ws::channel::{ticker::TickerEvent, Response};
use diesel::{prelude::*, r2d2::ConnectionManager, upsert::excluded, PgConnection};
use models::{candle::CandleBuilder, schema::candles, Candle};
use redis::Commands;
use tracing::error;
use types::Timeframe;

fn get_closed_candle(
    ticker: &String,
    open_time: chrono::DateTime<chrono::Utc>,
    timeframe: &Timeframe,
    redis_conn: &mut redis::Connection,
    pg_conn: &mut PgConnection,
) -> anyhow::Result<Option<Candle>> {
    let candle_exists = candles::table
        .filter(
            candles::pair
                .eq(ticker)
                .and(candles::open_time.eq(open_time))
                .and(candles::timeframe.eq(timeframe.to_string())),
        )
        .count()
        .get_result::<i64>(pg_conn)?
        != 0;
    if candle_exists {
        return Ok(None);
    }
    let last_closed_candle = candles::table
        .filter(
            candles::pair
                .eq(ticker)
                .and(candles::timeframe.eq(timeframe.to_string()))
                .and(candles::open_time.lt(open_time)),
        )
        .select(Candle::as_select())
        .order(candles::open_time.desc())
        .limit(1)
        .get_result(pg_conn)
        .optional()?;
    return Ok(last_closed_candle);
}

pub fn handle_ticker(
    payload: String,
    redis_pool: r2d2::Pool<redis::Client>,
    pg_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    is_backtest: bool,
) -> anyhow::Result<()> {
    let pg_conn = &mut pg_pool.get().context("Getting connection from pg_pool")?;
    let redis_conn = &mut redis_pool
        .get()
        .context("Getting connection from redis_pool")?;
    let data: Response<TickerEvent> =
        serde_json::from_str(&payload).context("Parsing redis message to Response<TickerEvent>")?;
    let timeframes = [
        Timeframe::Minute(2),
        Timeframe::Minute(5),
        Timeframe::Minute(15),
        Timeframe::Hour(1),
        Timeframe::Hour(4),
        Timeframe::Day(1),
        Timeframe::Week(1),
    ];

    for event in data.events() {
        for timeframe in timeframes.iter() {
            let (open_time, size_in_millis) = timeframe.open_and_size(data.timestamp())?;
            for ticker in event.tickers().iter() {
                match get_closed_candle(
                    ticker.product_id(),
                    open_time,
                    timeframe,
                    redis_conn,
                    pg_conn,
                ) {
                    Ok(Some(x)) => {
                        let channel = if is_backtest {
                            "backtest-candle_close"
                        } else {
                            "candle_close"
                        };
                        redis_conn
                            .publish(
                                channel,
                                serde_json::to_string(&x)
                                    .context("Stringify last_closed_candle for publishing on redis candle_close")?,
                            )
                            .context(format!("Publishing to redis {channel} channel"))?
                    }
                    Err(err) => error!("{err:#}"),
                    _ => (),
                }
                let candle = CandleBuilder::default()
                    .pair(ticker.product_id().to_owned())
                    .open_time(open_time)
                    .timeframe(timeframe.to_string())
                    .open(ticker.price().to_owned())
                    .high(ticker.price().to_owned())
                    .low(ticker.price().to_owned())
                    .close(ticker.price().to_owned())
                    .size_in_millis(size_in_millis)
                    .build()?;
                let result: Candle = diesel::insert_into(candles::table)
                    .values(candle)
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
                    .get_result(pg_conn)?;
                let channel = if is_backtest {
                    "backtest-candle"
                } else {
                    "candle"
                };
                redis_conn
                    .publish(
                        channel,
                        serde_json::to_string(&result).context(format!(
                            "Stringify result for publishing on redis {channel}"
                        ))?,
                    )
                    .context(format!("Publishing to redis {channel} channel"))?;
                println!("result: {result:#?}");
            }
        }
    }
    return Ok(());
}
