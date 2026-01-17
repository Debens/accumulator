use async_trait::async_trait;
use tokio::sync::watch;

use crate::types::inventory::Inventory;

#[async_trait]
pub trait InventorySource: Send + Sync {
    /// Returns a receiver that always holds the latest inventory snapshot.
    fn subscribe(&self) -> watch::Receiver<Inventory>;
}
