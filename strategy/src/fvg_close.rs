use anyhow::Context;
use diesel::{r2d2::ConnectionManager, PgConnection, RunQueryDsl};
use models::{fvg::FVG, trade::{TradeBuilder, Trade}, schema::trades};
use redis::Commands;

pub fn handle_fvg_close(
    payload: String,
    redis_pool: r2d2::Pool<redis::Client>,
    pg_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    is_backtest: bool,
) -> anyhow::Result<()> {
    let pg_conn = &mut pg_pool
        .get()
        .context("Getting connection from pg_pool")?;
    let redis_conn = &mut redis_pool
        .get()
        .context("Getting connection from redis_pool")?;
    let data: FVG = serde_json::from_str(&payload).context("Parsing redis message to Candle")?;
    let flow = if data.flow() == "bear" { "bull" } else { "bear" };
    let (entry, stop_loss) = if flow == "bull" {
        (data.high(), data.low())
    } else {
        (data.low(), data.high())
    };
    let risk = stop_loss - entry;
    let reward = rust_decimal::Decimal::from(2) * risk.abs();
    let take_profit = if flow == "bull" { entry + reward } else { entry - reward };
    let trade = TradeBuilder::default()
        .pair(data.pair().to_owned())
        .open_time(data.close_time().unwrap())
        .timeframe(data.timeframe().to_owned())
        .fill_time(None)
        .quantity(rust_decimal::Decimal::from(1000) / entry)
        .entry(entry.to_owned())
        .stop_loss(stop_loss.to_owned())
        .take_profit(take_profit.to_owned())
        .flow(flow.to_owned())
        .close_time(None)
        .close(None)
        .build()?;

    let result: Trade = diesel::insert_into(trades::table)
        .values(trade)
        .get_result(pg_conn)?;
    let channel = if is_backtest {
        "backtest-trade"
    } else {
        "trade"
    };
    redis_conn
        .publish(
            channel,
            serde_json::to_string(&result).context(format!(
                "Stringify result for publishing on redis {channel}"
            ))?,
        )
        .context(format!("Publishing to redis {channel} channel"))?;
    println!("new trade: {result:#?}");
    return Ok(());
}
