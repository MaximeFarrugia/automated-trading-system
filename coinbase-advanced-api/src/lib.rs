pub mod error;
pub mod signer;
pub mod ws;

use core::fmt;

pub use ws::WsClient;

pub enum CoinbaseService {
    Websocket,
    Rest,
}

impl fmt::Display for CoinbaseService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Websocket => "public_websocket_api",
            Self::Rest => "retail_rest_api_proxy",
        };
        return write!(f, "{value}");
    }
}
