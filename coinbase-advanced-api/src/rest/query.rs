use async_trait::async_trait;

use crate::error::Error;

use super::client::Client;

#[async_trait]
pub trait Query<T, C>
where
    C: Client,
{
    async fn query(&self, client: &C) -> Result<T, Error>;
}
