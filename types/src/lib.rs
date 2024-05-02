pub mod timeframe;

pub use timeframe::Timeframe;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid timeframe: {0}")]
    TimeframeError(String),
}
