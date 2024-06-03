use anyhow::Context;
use bytes::Bytes;

use async_trait::async_trait;
use http::{header::AUTHORIZATION, request, response, HeaderValue};
use url::Url;

use crate::{error::Error, signer::Signer};

#[async_trait]
pub trait Client {
    fn url(&self, endpoint: &str) -> Result<Url, Error>;

    async fn exec(
        &self,
        request: request::Builder,
        body: Vec<u8>,
        jwt_uri: String,
    ) -> Result<response::Response<Bytes>, Error>;
}

pub struct RestClient {
    client: reqwest::Client,
    base_url: Url,
    signer: Signer,
}

impl RestClient {
    pub fn new(key_name: &str, secret_key: &str) -> anyhow::Result<Self, Error> {
        let client = reqwest::Client::new();
        let base_url = Url::parse("https://api.coinbase.com/")?;

        return Ok(Self {
            client,
            base_url,
            signer: Signer::new(key_name, secret_key)?,
        });
    }

    async fn auth_req(
        &self,
        request: request::Builder,
        body: Vec<u8>,
        jwt_uri: String,
    ) -> anyhow::Result<response::Response<Bytes>, Error> {
        let jwt = self
            .signer
            .create_jwt(Some(jwt_uri.as_str()))
            .context("Creating JWT")?;
        let fct = || async {
            let mut http_request = request.body(body)?;
            let headers = http_request.headers_mut();
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(format!("Bearer {jwt}").as_str())?,
            );
            let request: reqwest::Request = http_request.try_into()?;
            let resp = self.client.execute(request).await?;
            let mut http_resp = http::response::Response::builder()
                .status(resp.status())
                .version(resp.version());

            if let Some(http_headers) = http_resp.headers_mut() {
                for (key, value) in resp.headers() {
                    http_headers.insert(key, value.to_owned());
                }
            }
            return Ok(http_resp.body(resp.bytes().await?)?);
        };

        return fct().await;
    }
}

#[async_trait]
impl Client for RestClient {
    fn url(&self, endpoint: &str) -> anyhow::Result<Url, Error> {
        let url = self.base_url.join(endpoint)?.to_owned();

        return Ok(url);
    }

    async fn exec(
        &self,
        request: request::Builder,
        body: Vec<u8>,
        jwt_uri: String,
    ) -> anyhow::Result<response::Response<Bytes>, Error> {
        return self.auth_req(request, body, jwt_uri).await;
    }
}
