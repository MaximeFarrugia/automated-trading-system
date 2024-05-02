use std::time::SystemTime;

use josekit::{
    jws::{
        alg::ecdsa::{EcdsaJwsAlgorithm::Es256, EcdsaJwsSigner},
        JwsHeader,
    },
    jwt::{self, JwtPayload},
};
use rand::Rng;

use crate::{error::Error, CoinbaseService};

pub struct Signer {
    signer: EcdsaJwsSigner,
    key_name: String,
}

impl Signer {
    pub fn new(key_name: &str, secret_key: &str) -> anyhow::Result<Self, Error> {
        return Ok(Self {
            signer: Es256.signer_from_pem(secret_key.as_bytes())?,
            key_name: key_name.to_owned(),
        });
    }

    pub fn create_jwt(&self, service: CoinbaseService) -> anyhow::Result<String, Error> {
        let mut header = JwsHeader::new();
        let mut payload = JwtPayload::new();
        let nonce: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(64)
            .map(char::from)
            .collect();
        let now = SystemTime::now();
        let exp = now + std::time::Duration::new(120, 0);

        header.set_algorithm("ES256");
        header.set_key_id(self.key_name.to_owned());
        header.set_nonce(nonce);
        header.set_token_type("JWT");
        payload.set_subject(self.key_name.to_owned());
        payload.set_issuer("coinbase-cloud".to_owned());
        payload.set_not_before(&now);
        payload.set_expires_at(&exp);
        payload.set_audience(vec![service.to_string()]);
        return Ok(jwt::encode_with_signer(&payload, &header, &self.signer)?);
    }
}
