use std::borrow::Cow;

use anyhow::Context;
use diesel::{prelude::*, r2d2::ConnectionManager, PgConnection};
use models::{
    schema::{candles, swings},
    Candle, swing::{SwingBuilder, Swing},
};
use redis::Commands;

use crate::candle_close::CandleCloseIndicator;

pub struct SwingIndicator {
    redis_pool: r2d2::Pool<redis::Client>,
    pg_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    is_backtest: bool,
}

impl SwingIndicator {
    pub fn new(
        redis_pool: r2d2::Pool<redis::Client>,
        pg_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
        is_backtest: bool,
    ) -> Self {
        return Self {
            redis_pool,
            pg_pool,
            is_backtest,
        };
    }

    fn get_last_candles(
        &self,
        candle: &Candle,
        pg_conn: &mut r2d2::PooledConnection<ConnectionManager<PgConnection>>,
    ) -> anyhow::Result<Vec<Candle>> {
        let candle = candles::table
            .filter(
                candles::pair
                    .eq(candle.pair())
                    .and(candles::timeframe.eq(candle.timeframe()))
                    .and(candles::open_time.lt(candle.open_time())),
            )
            .select(Candle::as_select())
            .order(candles::open_time.desc())
            .limit(4)
            .get_results(pg_conn)?;

        return Ok(candle);
    }

    fn handle_swing_creation(
        &self,
        candle: &Candle,
        pg_conn: &mut r2d2::PooledConnection<ConnectionManager<PgConnection>>,
    ) -> anyhow::Result<Option<Swing>> {
        let last_candles = self.get_last_candles(candle, pg_conn)?;
        if last_candles.len() != 4 {
            println!("wef: {candle:#?} {last_candles:#?}");
            return Ok(None);
        }
        let (first, second, third, fourth, fifth) = (&last_candles[3], &last_candles[2], &last_candles[1], &last_candles[0], candle);

        let mut swing_builder = SwingBuilder::default();
        if third.low() < first.low() && third.low() < second.low() && third.low() < fourth.low() && third.low() < fifth.low() {
            swing_builder
                .price(third.low().to_owned())
                .flow("bull".to_owned());
        } else if third.high() > first.high() && third.high() > second.high() && third.high() > fourth.high() && third.high() > fifth.high() {
            swing_builder
                .price(third.high().to_owned())
                .flow("bear".to_owned());
        } else {
            return Ok(None);
        }

        swing_builder
            .pair(candle.pair().to_owned())
            .open_time(third.open_time().to_owned())
            .timeframe(candle.timeframe().to_owned())
            .close_time(None);
        let swing = swing_builder.build()?;
        let result: Swing = diesel::insert_into(swings::table)
            .values(swing)
            .get_result(pg_conn)?;
        return Ok(Some(result));
    }

    fn handle_closed_swings(
        &self,
        candle: &Candle,
        pg_conn: &mut r2d2::PooledConnection<ConnectionManager<PgConnection>>,
    ) -> anyhow::Result<Vec<Swing>> {
        let swings = diesel::update(
            swings::table.filter(
                swings::pair
                    .eq(candle.pair())
                    .and(swings::timeframe.eq(candle.timeframe()))
                    .and(swings::open_time.lt(candle.open_time()))
                    .and(
                        swings::flow
                            .eq("bull")
                            .and(swings::price.gt(candle.close()))
                            .or(swings::flow.eq("bear").and(swings::price.lt(candle.close()))),
                    )
                    .and(swings::close_time.is_null()),
            ),
        )
        .set(swings::close_time.eq(candle.open_time()))
        .get_results(pg_conn)?;

        return Ok(swings);
    }

    fn publish_swings(
        &self,
        swings: Vec<Swing>,
        channel: Cow<'static, str>,
    ) -> anyhow::Result<()> {
        let redis_conn = &mut self
            .redis_pool
            .get()
            .context("Getting connection from redis_pool")?;
        let channel = if self.is_backtest {
            Cow::Owned(format!("backtest-{channel}"))
        } else {
            channel
        };
        for swing in swings.iter() {
            redis_conn
                .publish(
                    channel.to_string(),
                    serde_json::to_string(swing).context(format!(
                        "Stringify result for publishing on redis {channel}"
                    ))?,
                )
                .context(format!("Publishing to redis {channel} channel"))?;
        }
        return Ok(());
    }
}

impl CandleCloseIndicator for SwingIndicator {
    fn process(&self, candle: &Candle) -> anyhow::Result<()> {
        let pg_conn = &mut self
            .pg_pool
            .get()
            .context("Getting connection from pg_pool")?;
        let new_swing = self.handle_swing_creation(candle, pg_conn)?;
        let closed_swings = self.handle_closed_swings(candle, pg_conn)?;

        // println!("new_swing: {new_swing:#?} closed_swings: {closed_swings:#?}");
        if let Some(new_swing) = new_swing {
            self.publish_swings(vec![new_swing], Cow::Borrowed("swing")).context("publishing new_swing")?;
        }
        self.publish_swings(closed_swings, Cow::Borrowed("swing_close")).context("publishing closed_swings")?;
        return Ok(());
    }
}
