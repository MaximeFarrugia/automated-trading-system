use anyhow::Context;
use diesel::{r2d2::ConnectionManager, PgConnection};
use models::Candle;

use crate::fvg::FvgIndicator;

pub trait CandleCloseIndicator {
    fn process(&self, candle: &Candle) -> anyhow::Result<()>;
}

pub fn handle_candle_close(
    payload: String,
    redis_pool: r2d2::Pool<redis::Client>,
    pg_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    is_backtest: bool,
) -> anyhow::Result<()> {
    let data: Candle = serde_json::from_str(&payload).context("Parsing redis message to Candle")?;
    let fvg_indicator = FvgIndicator::new(redis_pool.clone(), pg_pool.clone(), is_backtest);
    let _ = fvg_indicator.process(&data)?;
    return Ok(());
}
