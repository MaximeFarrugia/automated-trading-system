use std::{
    borrow::Cow,
    collections::HashMap,
    ops::Deref,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use async_trait::async_trait;
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde::Serialize;
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

use crate::{error::Error, signer::Signer, CoinbaseService};

use super::channel::Channel;

#[async_trait]
pub trait Client {
    fn url(&self) -> Cow<'_, str> {
        return Cow::Borrowed("wss://advanced-trade-ws.coinbase.com");
    }

    async fn subscribe<T: Channel + Sync>(
        &mut self,
        channel: &T,
    ) -> anyhow::Result<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>, Error>;

    async fn unsubscribe<T: Channel + Sync>(&mut self, channel: &T) -> anyhow::Result<(), Error>;
}

pub struct WsClient {
    signer: Signer,
    connections: HashMap<String, SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
}

impl WsClient {
    pub fn new(key_name: &str, secret_key: &str) -> anyhow::Result<Self, Error> {
        return Ok(Self {
            signer: Signer::new(key_name, secret_key)?,
            connections: HashMap::new(),
        });
    }

    pub(crate) async fn toggle_heartbeats(
        &self,
        sink: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
        subscribe: bool,
        product_id: Cow<'_, str>,
    ) -> anyhow::Result<(), Error> {
        let request = Request {
            r#type: format!(
                "{}",
                if subscribe {
                    "subscribe"
                } else {
                    "unsubscribe"
                }
            ),
            product_ids: vec![product_id.into_owned()],
            channel: "heartbeats".to_owned(),
            jwt: self
                .signer
                .create_jwt(CoinbaseService::Websocket)
                .context("Creating JWT")?,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        sink.send(Message::text(
            serde_json::to_string::<Request>(&request)
                .context("Stringify toggle_heartbeat request")?,
        ))
        .await
        .context("Sending toggle_heartbeat request to coinbase")?;
        return Ok(());
    }
}

#[derive(Serialize)]
struct Request {
    r#type: String,
    product_ids: Vec<String>,
    channel: String,
    jwt: String,
    timestamp: u64,
}

#[async_trait]
impl Client for WsClient {
    async fn subscribe<T: Channel + Sync>(
        &mut self,
        channel: &T,
    ) -> anyhow::Result<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>, Error> {
        let url = self.url();
        let conn_id = format!("{}-{}", channel.name(), channel.product_id());
        let (socket, _) = tokio_tungstenite::connect_async(url.deref()).await?;

        let (mut sink, stream) = socket.split();
        self.toggle_heartbeats(&mut sink, true, channel.product_id())
            .await?;
        if channel.name().deref() != "heartbeats" {
            let request = Request {
                r#type: "subscribe".to_owned(),
                product_ids: vec![channel.product_id().into_owned()],
                channel: channel.name().into_owned(),
                jwt: self.signer.create_jwt(CoinbaseService::Websocket)?,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            sink.send(Message::text(serde_json::to_string::<Request>(&request)?))
                .await?;
        }
        self.connections.insert(conn_id, sink);
        return Ok(stream);
    }

    async fn unsubscribe<T: Channel + Sync>(&mut self, channel: &T) -> anyhow::Result<(), Error> {
        let conn_id = format!("{}-{}", channel.name(), channel.product_id());
        let sink = self.connections.remove(&conn_id);

        if let Some(mut sink) = sink {
            self.toggle_heartbeats(&mut sink, false, channel.product_id())
                .await?;
            if channel.name().deref() != "heartbeats" {
                let request = Request {
                    r#type: "unsubscribe".to_owned(),
                    product_ids: vec![channel.product_id().into_owned()],
                    channel: channel.name().into_owned(),
                    jwt: self.signer.create_jwt(CoinbaseService::Websocket)?,
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                };
                sink.send(Message::text(serde_json::to_string::<Request>(&request)?))
                    .await?;
            }
            sink.close().await?;
        }
        return Ok(());
    }
}
