use std::borrow::Cow;

use async_trait::async_trait;
use http::{header::CONTENT_TYPE, Method};
use serde::de::DeserializeOwned;

use crate::error::Error;

use super::{client::Client, params::QueryParams, query::Query};

pub trait Endpoint {
    fn method(&self) -> Method {
        return Method::GET;
    }

    fn endpoint(&self) -> Cow<'static, str>;

    fn body(&self) -> Result<Option<(&'static str, Vec<u8>)>, Error> {
        return Ok(None);
    }

    fn params(&self) -> QueryParams {
        return QueryParams::default();
    }
}

#[async_trait]
impl<E, T, C> Query<T, C> for E
where
    E: Endpoint + Sync,
    T: DeserializeOwned,
    C: Client + Sync,
{
    async fn query(&self, client: &C) -> Result<T, Error> {
        let mut url = client.url(&self.endpoint())?;
        let jwt_uri = format!("{} api.coinbase.com/{}", self.method(), self.endpoint());
        self.params().add_to_url(&mut url);

        let request = http::request::Request::builder()
            .method(self.method())
            .uri(url.as_str());
        let (request, body) = match self.body()? {
            Some((content_type, body)) => {
                let request = request.header(CONTENT_TYPE, content_type);
                (request, body)
            }
            None => (request, Vec::new()),
        };
        let resp = client.exec(request, body, jwt_uri).await?;
        let json_value = serde_json::from_slice(resp.body())?;
        let res = serde_json::from_value::<T>(json_value)?;

        return Ok(res);
    }
}
