pub mod state;

use anyhow::Context;
use async_trait::async_trait;
use models::{fvg::FVG, swing::Swing};
use statig::prelude::IntoStateMachineExt;

use crate::AppState;

use self::state::Event;

use super::Strategy;

pub struct Combo {
    state: AppState,
    v1: Option<FVG>,
    v2: Option<FVG>,
    v3: Option<Swing>,
    v4: Option<FVG>,
}

impl Combo {
    pub fn new(state: AppState) -> Self {
        return Self {
            state,
            v1: None,
            v2: None,
            v3: None,
            v4: None,
        };
    }

}

#[async_trait]
impl Strategy for Combo {
    async fn run(state: AppState) -> anyhow::Result<()> {
        let mut redis_sub_conn = state.redis_pool
            .get()
            .context("Get redis_sub_conn from redis_pool")?;
        let mut pg_conn = state.pg_pool
            .get()
            .context("Get pg_conn from pg_pool")?;
        let mut pubsub = redis_sub_conn.as_pubsub();
        let mut state_machine = Self::new(state).state_machine();

        pubsub.subscribe("fvg")?;
        pubsub.subscribe("backtest-fvg")?;
        pubsub.subscribe("swing")?;
        pubsub.subscribe("backtest-swing")?;
        pubsub.subscribe("candle_close")?;
        pubsub.subscribe("backtest-candle_close")?;
        loop {
            let msg = pubsub.get_message()?;
            let channel: String = msg
                .get_channel()
                .context("channel from redis pubsub message")?;
            let payload: String = msg.get_payload()?;

            let res = match channel.as_str() {
                "fvg" => Ok(state_machine.handle(&Event::Fvg(serde_json::from_str(&payload).context("Parsing redis message to FVG")?))),
                "backtest-fvg" => Ok(state_machine.handle(&Event::Fvg(serde_json::from_str(&payload).context("Parsing redis message to FVG")?))),
                "swing" => Ok(state_machine.handle(&Event::Swing(serde_json::from_str(&payload).context("Parsing redis message to Swing")?))),
                "backtest-swing" => Ok(state_machine.handle(&Event::Swing(serde_json::from_str(&payload).context("Parsing redis message to Swing")?))),
                "candle_close" => Ok(state_machine.handle(&Event::CandleClose(serde_json::from_str(&payload).context("Parsing redis message to Candle")?))),
                "backtest-candle_close" => Ok(state_machine.handle(&Event::CandleClose(serde_json::from_str(&payload).context("Parsing redis message to Candle")?))),
                _ => Err(anyhow::anyhow!("No handler for redis channel {channel}")),
            };

            if let Err(err) = res {
                tracing::error!("{err:#}");
            }
        }
    }
}
