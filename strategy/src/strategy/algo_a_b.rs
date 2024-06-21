use std::borrow::Cow;

use anyhow::{Context, bail};
use async_trait::async_trait;
use models::fvg::FVG;
use redis::Commands;
use types::Timeframe;

use crate::AppState;

use super::Strategy;

pub struct AlgoABStrat {
    state: AppState,
    fvg_1d: Option<FVG>,
    fvg_1h: Option<FVG>,
    fvg_5m: Option<FVG>,
}

impl AlgoABStrat {
    pub fn new(state: AppState) -> Self {
        return Self {
            state,
            fvg_1d: None,
            fvg_1h: None,
            fvg_5m: None,
        };
    }

    fn publish_fvg(
        &self,
        fvg: &FVG,
        channel: Cow<'static, str>,
        is_backtest: bool,
    ) -> anyhow::Result<()> {
        let redis_conn = &mut self
            .state
            .redis_pool
            .get()
            .context("Getting connection from redis_pool")?;
        let channel = if is_backtest {
            Cow::Owned(format!("backtest-{channel}"))
        } else {
            channel
        };
        redis_conn
            .publish(
                channel.to_string(),
                serde_json::to_string(fvg).context(format!(
                    "Stringify result for publishing on redis {channel}"
                ))?,
            )
            .context(format!("Publishing to redis {channel} channel"))?;
        return Ok(());
    }

    pub fn handle_fvg(&mut self, payload: String, is_backtest: bool) -> anyhow::Result<()> {
        let data: FVG = serde_json::from_str(&payload).context("Parsing redis message to FVG")?;

        let fvg = if data.timeframe() == &Timeframe::Day(1).to_string() {
            let fvg = match self.fvg_1d.as_ref() {
                None => Some(data),
                Some(fvg) if fvg.open_time() < data.open_time() => Some(data),
                _ => None,
            };
            if fvg.is_some() {
                self.fvg_1d = fvg;
            }
            self.fvg_1d.as_ref()
        } else if data.timeframe() == &Timeframe::Hour(1).to_string() {
            let fvg = match self.fvg_1h.as_ref() {
                None => Some(data),
                Some(fvg) if fvg.open_time() < data.open_time() => Some(data),
                _ => None,
            };
            if fvg.is_some() {
                self.fvg_1h = fvg;
            }
            self.fvg_1h.as_ref()
        } else if data.timeframe() == &Timeframe::Minute(5).to_string() {
            let fvg = match self.fvg_5m.as_ref() {
                None => Some(data),
                Some(fvg) if fvg.open_time() < data.open_time() => Some(data),
                _ => None,
            };
            if fvg.is_some() {
                self.fvg_5m = fvg;
            }
            self.fvg_5m.as_ref()
        } else {
            None
        };
        if let Some(fvg) = fvg {
            println!("{fvg:#?}");
            self.publish_fvg(fvg, Cow::Borrowed("strategy_fvg"), is_backtest)?;
        }
        return Ok(());
    }

    async fn handle_redis_message(&mut self, msg: redis::Msg) -> anyhow::Result<()> {
        let channel: String = msg
            .get_channel()
            .context("channel from redis pubsub message")?;
        let payload: String = msg.get_payload()?;

        match channel.as_str() {
            "fvg" => self.handle_fvg(payload, false)?,
            "backtest-fvg" => self.handle_fvg(payload, true)?,
            _ => bail!("No handler for redis channel {channel}"),
        };
        return Ok(());
    }
}

#[async_trait]
impl Strategy for AlgoABStrat {
    async fn run(state: AppState) -> anyhow::Result<()> {
        let mut redis_sub_conn = state.redis_pool
            .get()
            .context("Get redis_sub_conn from redis_pool")?;
        let mut pg_conn = state.pg_pool
            .get()
            .context("Get pg_conn from pg_pool")?;
        let mut pubsub = redis_sub_conn.as_pubsub();
        let mut strat = AlgoABStrat::new(state);

        pubsub.subscribe("fvg")?;
        pubsub.subscribe("backtest-fvg")?;
        loop {
            let msg = pubsub.get_message()?;

            let res = strat.handle_redis_message(msg).await;

            if let Err(err) = res {
                tracing::error!("{err:#}");
            }
        }
    }
}
