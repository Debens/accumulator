use crate::risk::{context::RiskContext, decision::RiskReason, engine::RiskCheck};

pub struct InventoryAvailableCheck;

impl InventoryAvailableCheck {
    pub fn new() -> Self {
        Self
    }
}

impl RiskCheck for InventoryAvailableCheck {
    fn name(&self) -> &'static str {
        "InventoryAvailableCheck"
    }

    fn evaluate(&mut self, ctx: &RiskContext) -> Result<(), Vec<RiskReason>> {
        let mut reasons = Vec::new();

        if let Some(bid) = ctx.target.bid {
            let required = bid.price.as_f64() * bid.quantity;
            if required > ctx.inventory.quote {
                reasons.push(RiskReason::InsufficientInventory {
                    asset: ctx.instrument.quote().to_string(),
                    required,
                    available: ctx.inventory.quote,
                });
            }
        }

        if let Some(ask) = ctx.target.ask {
            let required = ask.quantity;
            if required > ctx.inventory.base {
                reasons.push(RiskReason::InsufficientInventory {
                    asset: ctx.instrument.base().to_string(),
                    required,
                    available: ctx.inventory.base,
                });
            }
        }

        if reasons.is_empty() {
            Ok(())
        } else {
            Err(reasons)
        }
    }
}
