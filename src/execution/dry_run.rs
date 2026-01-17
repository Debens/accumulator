use async_trait::async_trait;
use rand::Rng;
use tokio::sync::broadcast;

use anyhow::Result;
use tracing::info;

use crate::{
    execution::{
        DynamicInventorySource, ExecutionVenue, ReportSender, order_action::OrderAction,
        order_report::OrderReport, types::OpenOrder,
    },
    kraken::kraken_inventory::KrakenInventory,
    types::instrument::Instrument,
};

#[derive(Debug)]
pub struct DryRunExecutionVenue {
    on_report: Option<broadcast::Sender<OrderReport>>,
}

impl Default for DryRunExecutionVenue {
    fn default() -> Self {
        Self { on_report: None }
    }
}

impl DryRunExecutionVenue {
    pub fn new(on_report: broadcast::Sender<OrderReport>) -> Self {
        Self {
            on_report: Some(on_report),
        }
    }

    async fn emit(&self, report: OrderReport) {
        if let Some(sender) = &self.on_report {
            info!(?report);
            let _ = sender.send(report);
        };
    }
}

#[async_trait]
impl ExecutionVenue for DryRunExecutionVenue {
    async fn open_orders(&self, _instrument: &Instrument) -> Result<Vec<OpenOrder>> {
        Ok(Vec::new())
    }

    async fn spawn_reports(&self, on_report: ReportSender) -> Result<()> {
        Ok(())
    }

    async fn spawn_inventory(&self, instrument: &Instrument) -> Result<DynamicInventorySource> {
        let inventory = KrakenInventory::spawn(instrument).await?;

        Ok(Box::new(inventory))
    }

    async fn execute(&self, actions: &[OrderAction]) -> Result<()> {
        for action in actions {
            match action {
                OrderAction::CancelAll => {
                    info!("cancelling all orders");

                    /* NOTE: nothing to be done right now, all orders fill immediately in a dry run */
                }
                OrderAction::Cancel {
                    order_id,
                    instrument,
                    side,
                } => {
                    let cancel = OrderReport::Cancel {
                        order_id: order_id.clone(),
                        instrument: instrument.clone(),
                        side: *side,
                    };

                    self.emit(cancel).await;

                    let cancelled = OrderReport::Cancelled {
                        order_id: order_id.clone(),
                        instrument: instrument.clone(),
                        side: *side,
                    };

                    self.emit(cancelled).await;
                }
                OrderAction::Place(place) => {
                    let will_reject = {
                        let mut rng = rand::rng();
                        rng.random_range(0..10)
                    };

                    let placed = OrderReport::Placed {
                        order_id: place.order_id.clone(),
                        instrument: place.instrument.clone(),
                        side: place.side,
                        price: place.price,
                        quantity: place.quantity,
                    };

                    self.emit(placed).await;

                    let outcome = match will_reject {
                        0 => OrderReport::Rejected {
                            order_id: place.order_id.clone(),
                            instrument: place.instrument.clone(),
                            side: place.side,
                            reason: "rejected".to_string(),
                        },
                        _ => OrderReport::Accepted {
                            order_id: place.order_id.clone(),
                            instrument: place.instrument.clone(),
                            side: place.side,
                            price: place.price,
                            quantity: place.quantity,
                        },
                    };

                    self.emit(outcome).await;
                }
            };
        }
        Ok(())
    }
}
