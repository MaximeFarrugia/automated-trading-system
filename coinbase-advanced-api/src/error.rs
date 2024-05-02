#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),

    #[error(transparent)]
    JoseError(#[from] josekit::JoseError),

    #[error(transparent)]
    TungsteniteError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
}
