pub mod dry_run;
pub mod order_action;
pub mod order_manager;
pub mod order_report;
pub mod order_side_manager;
pub mod types;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::broadcast;

use crate::execution::order_action::OrderAction;
use crate::execution::order_report::OrderReport;
use crate::execution::types::OpenOrder;
use crate::inventory::InventorySource;
use crate::types::instrument::Instrument;

pub type ReportSender = broadcast::Sender<OrderReport>;

pub type DynamicInventorySource = Box<dyn InventorySource + Send + Sync>;

#[async_trait]
pub trait ExecutionVenue {
    async fn execute(&self, actions: &[OrderAction]) -> Result<()>;
    async fn open_orders(&self, instrument: &Instrument) -> Result<Vec<OpenOrder>>;
    async fn spawn_reports(&self, on_report: ReportSender) -> Result<()>;
    async fn spawn_inventory(&self, instrument: &Instrument) -> Result<DynamicInventorySource>;
}
