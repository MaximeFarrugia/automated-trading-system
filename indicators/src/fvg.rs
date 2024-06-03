use anyhow::Context;
use diesel::{prelude::*, r2d2::ConnectionManager, PgConnection};
use models::{
    fvg::{FVGBuilder, FVG},
    schema::{candles, fvgs},
    Candle,
};

use crate::candle_close::CandleCloseIndicator;

pub struct FvgIndicator {
    redis_pool: r2d2::Pool<redis::Client>,
    pg_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}

impl FvgIndicator {
    pub fn new(
        redis_pool: r2d2::Pool<redis::Client>,
        pg_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
    ) -> Self {
        return Self {
            redis_pool,
            pg_pool,
        };
    }

    fn get_last_candle(
        &self,
        candle: &Candle,
        pg_conn: &mut r2d2::PooledConnection<ConnectionManager<PgConnection>>,
    ) -> anyhow::Result<Option<Candle>> {
        let candle = candles::table
            .filter(
                candles::pair
                    .eq(candle.pair())
                    .and(candles::timeframe.eq(candle.timeframe()))
                    .and(candles::open_time.lt(candle.open_time())),
            )
            .select(Candle::as_select())
            .order(candles::open_time.desc())
            .limit(1)
            .offset(1)
            .get_result(pg_conn)
            .optional()?;

        return Ok(candle);
    }

    fn handle_fvg_creation(
        &self,
        candle: &Candle,
        pg_conn: &mut r2d2::PooledConnection<ConnectionManager<PgConnection>>,
    ) -> anyhow::Result<Option<FVG>> {
        let last_candle = self.get_last_candle(candle, pg_conn)?;
        if last_candle.is_none() {
            return Ok(None);
        }

        let last_candle = last_candle.unwrap();
        let mut fvg_builder = FVGBuilder::default();
        if last_candle.high() < candle.low() {
            fvg_builder
                .high(candle.low().to_owned())
                .low(last_candle.high().to_owned())
                .flow("bull".to_owned());
        } else if last_candle.low() > candle.high() {
            fvg_builder
                .high(last_candle.low().to_owned())
                .low(candle.high().to_owned())
                .flow("bear".to_owned());
        } else {
            return Ok(None);
        }

        fvg_builder
            .pair(candle.pair().to_owned())
            .open_time(last_candle.open_time().to_owned())
            .timeframe(candle.timeframe().to_owned())
            .close_time(None);
        let fvg = fvg_builder.build()?;
        let result: FVG = diesel::insert_into(fvgs::table)
            .values(fvg)
            .get_result(pg_conn)?;
        return Ok(Some(result));
    }

    fn handle_closed_fvgs(
        &self,
        candle: &Candle,
        pg_conn: &mut r2d2::PooledConnection<ConnectionManager<PgConnection>>,
    ) -> anyhow::Result<Vec<FVG>> {
        let fvgs = diesel::update(
            fvgs::table.filter(
                fvgs::pair
                    .eq(candle.pair())
                    .and(fvgs::timeframe.eq(candle.timeframe()))
                    .and(fvgs::open_time.lt(candle.open_time()))
                    .and(
                        fvgs::flow
                            .eq("bull")
                            .and(fvgs::low.gt(candle.close()))
                            .or(fvgs::flow.eq("bear").and(fvgs::high.lt(candle.close()))),
                    )
                    .and(fvgs::close_time.is_null()),
            ),
        )
        .set(fvgs::close_time.eq(candle.open_time()))
        .get_results(pg_conn)?;

        return Ok(fvgs);
    }
}

impl CandleCloseIndicator for FvgIndicator {
    fn process(&self, candle: &Candle) -> anyhow::Result<()> {
        let pg_conn = &mut self
            .pg_pool
            .get()
            .context("Getting connection from pg_pool")?;
        let new_fvg = self.handle_fvg_creation(candle, pg_conn)?;
        let closed_fvgs = self.handle_closed_fvgs(candle, pg_conn)?;
        println!("new_fvg: {new_fvg:#?} closed_fvgs: {closed_fvgs:#?}");
        return Ok(());
    }
}
