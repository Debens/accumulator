use std::time::Duration;

use crate::risk::context::RiskContext;
use crate::risk::decision::RiskReason;
use crate::risk::engine::RiskCheck;

#[derive(Debug, Clone)]
pub struct MarketFreshnessCheck {
    pub max_staleness: Duration,
}

impl MarketFreshnessCheck {
    pub fn new(max_staleness: Duration) -> Self {
        Self { max_staleness }
    }
}

impl RiskCheck for MarketFreshnessCheck {
    fn name(&self) -> &'static str {
        "MarketFreshnessCheck"
    }

    fn evaluate(&mut self, context: &RiskContext) -> Result<(), Vec<RiskReason>> {
        if context.market_state.is_stale(self.max_staleness) {
            return Err(vec![RiskReason::MarketDataStale]);
        }
        Ok(())
    }
}
