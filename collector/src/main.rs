use std::{borrow::Cow, future};

use anyhow::Context;
use coinbase_advanced_api::{
    ws::{
        channel::{
            ticker::{TickerChannel, TickerChannelBuilder, TickerEvent},
            Channel, Response,
        },
        client::Client,
    },
    WsClient,
};
use futures::StreamExt;
use redis::Commands;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    let api_key = std::env::var("CB_API_KEY").context("CB_API_KEY from .env file")?;
    let private_key = std::env::var("CB_PRIVATE_KEY").context("CB_PRIVATE_KEY from .env file")?;
    let mut client = WsClient::new(&api_key, &private_key)?;
    let btc_usd_ticker = TickerChannelBuilder::default()
        .product_id(Cow::Borrowed("BTC-USD"))
        .build()?;
    let redis_url = std::env::var("REDIS_URL").context("REDIS_URL from .env file")?;
    let redis_client = redis::Client::open(redis_url)?;
    let mut redis_conn = redis_client.get_connection()?;

    let stream = client.subscribe(&btc_usd_ticker).await?;
    let wef = tokio::spawn(async move {
        let mut publish = |message: tokio_tungstenite::tungstenite::Message| -> anyhow::Result<()> {
            let ticker = TickerChannel::parse(message).context("Parsing TickerChannel message")?;
            let json_message = serde_json::to_string::<Response<TickerEvent>>(&ticker)?;
            println!("{json_message}");
            let _: () = redis_conn
                .publish("ticker".to_owned(), json_message)
                .context("Publishing to redis ticker channel")?;
            Ok(())
        };
        stream
            .for_each(|x| {
                let res = match x.context("Received from websocket") {
                    Ok(message) => publish(message),
                    Err(err) => Err(err),
                };
                if let Err(err) = res {
                    println!("{err:#}");
                }
                future::ready(())
            })
            .await;
    });
    // tokio::time::sleep(tokio::time::Duration::new(20, 0)).await;
    // client.unsubscribe(&btc_usd_ticker).await?;
    wef.await?;
    return Ok(());
}
