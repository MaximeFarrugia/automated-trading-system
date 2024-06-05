use anyhow::Context;
use diesel::{prelude::*, r2d2::ConnectionManager, PgConnection, RunQueryDsl};
use models::{trade::{TradeBuilder, Trade}, schema::trades, Candle};
use redis::Commands;

pub fn handle_candle(
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
    let data: Candle = serde_json::from_str(&payload).context("Parsing redis message to Candle")?;
    let trades: Vec<Trade> = diesel::update(
        trades::table.filter(
            trades::pair
                .eq(data.pair())
                .and(trades::timeframe.eq(data.timeframe()))
                .and(trades::fill_time.is_null())
                .and(trades::open_time.lt(data.open_time()))
                .and(
                    trades::flow
                        .eq("bull")
                        .and(trades::entry.ge(data.close()))
                        .or(trades::flow.eq("bear").and(trades::entry.le(data.close()))),
                )
        )
    )
    .set(trades::fill_time.eq(data.open_time()))
    .get_results(pg_conn)?;
    if !trades.is_empty() {
        println!("filled trades: {trades:#?}");
    }
    let trades: Vec<Trade> = diesel::update(
        trades::table.filter(
            trades::pair
                .eq(data.pair())
                .and(trades::timeframe.eq(data.timeframe()))
                .and(trades::close_time.is_null())
                .and(trades::fill_time.le(data.open_time()))
                .and(
                    trades::flow
                        .eq("bull")
                        .and(trades::take_profit.le(data.close()))
                        .or(trades::flow.eq("bear").and(trades::take_profit.ge(data.close())))
                )
        )
    )
    .set((
        trades::close_time.eq(data.open_time()),
        trades::close.eq(trades::take_profit.nullable()),
    ))
    .get_results(pg_conn)?;
    if !trades.is_empty() {
        println!("closed trades TP: {trades:#?}");
    }
    let trades: Vec<Trade> = diesel::update(
        trades::table.filter(
            trades::pair
                .eq(data.pair())
                .and(trades::timeframe.eq(data.timeframe()))
                .and(trades::close_time.is_null())
                .and(trades::fill_time.le(data.open_time()))
                .and(
                    trades::flow
                        .eq("bull")
                        .and(trades::stop_loss.ge(data.close()))
                        .or(trades::flow.eq("bear").and(trades::stop_loss.le(data.close())))
                )
        )
    )
    .set((
        trades::close_time.eq(data.open_time()),
        trades::close.eq(trades::stop_loss.nullable()),
    ))
    .get_results(pg_conn)?;
    if !trades.is_empty() {
        println!("closed trades SL: {trades:#?}");
    }
    return Ok(());
}
