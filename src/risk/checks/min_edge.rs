use crate::{
    risk::{context::RiskContext, decision::RiskReason, engine::RiskCheck},
    types::instrument::Instrument,
};

pub struct MinEdgeCheck {
    pub min_half_spread: f64,
}

impl MinEdgeCheck {
    pub fn new(min_half_spread: f64) -> Self {
        Self { min_half_spread }
    }

    pub fn for_instrument(instrument: &Instrument) -> Self {
        Self::new(instrument.trading_rules().min_half_spread)
    }
}

impl RiskCheck for MinEdgeCheck {
    fn name(&self) -> &'static str {
        "MinEdgeCheck"
    }

    fn evaluate(&mut self, ctx: &RiskContext) -> Result<(), Vec<RiskReason>> {
        let best_bid = ctx
            .market_state
            .best_bid()
            .ok_or_else(|| vec![RiskReason::MissingMarketData])?;

        let best_ask = ctx
            .market_state
            .best_ask()
            .ok_or_else(|| vec![RiskReason::MissingMarketData])?;

        let spread = best_ask - best_bid;
        let half = spread / 2.0;

        if half < self.min_half_spread {
            return Err(vec![RiskReason::InsufficientEdge {
                half_spread: half,
                required: self.min_half_spread,
            }]);
        }

        Ok(())
    }
}
