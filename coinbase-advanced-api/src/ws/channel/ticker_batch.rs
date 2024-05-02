use std::borrow::Cow;

use derive_builder::Builder;

use super::Channel;

#[derive(Debug, Builder)]
pub struct TickerBatchChannel<'a> {
    product_id: Cow<'a, str>,
}

impl<'a> Channel for TickerBatchChannel<'a> {
    fn name(&self) -> Cow<'_, str> {
        return Cow::Borrowed("ticker_batch");
    }

    fn product_id(&self) -> Cow<'_, str> {
        return self.product_id.clone();
    }
}
