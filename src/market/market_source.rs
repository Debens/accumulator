use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

use crate::events::MarketEvent;
use crate::types::instrument::Instrument;

#[async_trait]
pub trait MarketDataSource: Send + Sync {
    async fn subscribe(&self, instrument: &Instrument, channel: Sender<MarketEvent>) -> Result<()>;
}
