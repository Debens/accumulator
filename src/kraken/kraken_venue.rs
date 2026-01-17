use async_trait::async_trait;
use tokio::sync::broadcast;

use anyhow::Result;

use crate::{
    execution::{
        DynamicInventorySource, ExecutionVenue, ReportSender,
        order_action::{OrderAction, OrderType},
        order_report::OrderReport,
        types::OpenOrder,
    },
    kraken::{
        kraken_client::KrakenClient, kraken_config::KrakenConfig,
        kraken_executions::KrakenExecutions, kraken_inventory::KrakenInventory,
    },
    types::instrument::Instrument,
};

#[derive(Debug, Clone)]
pub struct KrakenExecutionVenue {
    client: KrakenClient,
    on_report: Option<broadcast::Sender<OrderReport>>,
}

impl KrakenExecutionVenue {
    pub fn new(config: KrakenConfig, on_report: broadcast::Sender<OrderReport>) -> Self {
        Self {
            client: KrakenClient::new(config),
            on_report: Some(on_report),
        }
    }

    async fn emit(&self, report: OrderReport) {
        if let Some(sender) = &self.on_report {
            let _ = sender.send(report);
        }
    }
}

#[async_trait]
impl ExecutionVenue for KrakenExecutionVenue {
    async fn open_orders(&self, _instrument: &Instrument) -> Result<Vec<OpenOrder>> {
        todo!()
    }

    async fn spawn_inventory(&self, instrument: &Instrument) -> Result<DynamicInventorySource> {
        let inventory = KrakenInventory::spawn(instrument).await?;

        Ok(Box::new(inventory))
    }

    async fn spawn_reports(&self, on_report: ReportSender) -> Result<()> {
        KrakenExecutions::spawn(on_report).await?;

        Ok(())
    }

    async fn execute(&self, actions: &[OrderAction]) -> Result<()> {
        for action in actions {
            match action {
                OrderAction::CancelAll => {
                    tracing::warn!("cancelling all orders on venue");

                    let outcome = match self.client.cancel_all_orders().await {
                        Ok(result) => {
                            tracing::warn!(count = result.count, "cancel all complete");
                            OrderReport::CancelledAll {
                                count: result.count,
                            }
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "cancel all failed");
                            OrderReport::VenueError {
                                message: format!("cancel all failed: {e}"),
                            }
                        }
                    };

                    self.emit(outcome).await;
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

                    let outcome = match self.client.cancel_order(order_id).await {
                        Ok(res) if res.count > 0 => OrderReport::Cancelled {
                            order_id: order_id.clone(),
                            instrument: instrument.clone(),
                            side: *side,
                        },
                        Ok(_) => OrderReport::CancelFailed {
                            order_id: order_id.clone(),
                            instrument: instrument.clone(),
                            side: *side,
                            reason: "cancel returned 0 orders".to_string(),
                        },
                        Err(e) => {
                            let message = e.to_string().to_lowercase();
                            if message.contains("unknown order") {
                                OrderReport::Cancelled {
                                    order_id: order_id.clone(),
                                    instrument: instrument.clone(),
                                    side: *side,
                                }
                            } else {
                                OrderReport::VenueError {
                                    message: format!("cancel order {order_id} failed: {e}"),
                                }
                            }
                        }
                    };

                    self.emit(outcome).await;
                }

                OrderAction::Place(place) => {
                    let placed = OrderReport::Placed {
                        order_id: place.order_id.clone(),
                        instrument: place.instrument.clone(),
                        side: place.side,
                        price: place.price,
                        quantity: place.quantity,
                    };

                    self.emit(placed).await;

                    let result = match place.order_type {
                        OrderType::PostOnlyLimit => {
                            self.client
                                .limit_order(
                                    &place.instrument,
                                    place.side,
                                    place.price,
                                    place.quantity,
                                    &place.order_id,
                                )
                                .await
                        }
                    };

                    let outcome = match result {
                        Ok(_) => OrderReport::Accepted {
                            order_id: place.order_id.clone(),
                            instrument: place.instrument.clone(),
                            side: place.side,
                            price: place.price,
                            quantity: place.quantity,
                        },
                        Err(error) => OrderReport::Rejected {
                            order_id: place.order_id.clone(),
                            instrument: place.instrument.clone(),
                            side: place.side,
                            reason: error.to_string(),
                        },
                    };

                    self.emit(outcome).await;
                }
            }
        }

        Ok(())
    }
}
