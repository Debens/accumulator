use crate::risk::context::RiskContext;
use crate::risk::decision::RiskReason;
use crate::risk::engine::RiskCheck;

#[derive(Debug, Clone)]
pub struct MarketSanityCheck;

impl MarketSanityCheck {
    pub fn new() -> Self {
        Self
    }
}

impl RiskCheck for MarketSanityCheck {
    fn name(&self) -> &'static str {
        "MarketSanityCheck"
    }

    fn evaluate(&mut self, context: &RiskContext) -> Result<(), Vec<RiskReason>> {
        let best_bid = context.market_state.best_bid().map(|price| price.as_f64());
        let best_ask = context.market_state.best_ask().map(|price| price.as_f64());

        match (best_bid, best_ask) {
            (Some(bid), Some(ask))
                if bid.is_finite() && ask.is_finite() && bid > 0.0 && ask > 0.0 && bid < ask =>
            {
                Ok(())
            }
            _ => Err(vec![RiskReason::CrossedOrInvalidBook]),
        }
    }
}
