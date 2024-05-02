use async_trait::async_trait;

use crate::error::Error;

use super::client::Client;

#[async_trait]
pub trait Request<C>
where
    C: Client,
{
    async fn subscribe(&self, client: &C) -> anyhow::Result<(), Error>;

    fn unsubscribe(&self) -> anyhow::Result<(), Error>;
}
