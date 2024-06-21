pub mod algo_a_b;

use async_trait::async_trait;

use crate::AppState;

#[async_trait]
pub trait Strategy {
    async fn run(state: AppState) -> anyhow::Result<()>;
}
