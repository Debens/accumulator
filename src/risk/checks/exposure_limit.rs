use crate::{
    execution::order_action::Side,
    risk::{context::RiskContext, decision::RiskReason, engine::RiskCheck},
};

pub struct ExposureLimitCheck {
    max_exposure_in_quote: f64,
}

impl ExposureLimitCheck {
    pub fn new(max_exposure_in_quote: f64) -> Self {
        Self {
            max_exposure_in_quote,
        }
    }
}

impl RiskCheck for ExposureLimitCheck {
    fn name(&self) -> &'static str {
        "ExposureLimitCheck"
    }

    fn evaluate(&mut self, ctx: &RiskContext) -> Result<(), Vec<RiskReason>> {
        let mid = ctx
            .market_state
            .mid_price()
            .ok_or_else(|| vec![RiskReason::MissingMarketData])?;

        let mut reasons = Vec::new();

        if let Some(bid) = ctx.target.bid {
            let projected_base = ctx.inventory.base + bid.quantity;
            let exposure_quote = projected_base * mid.as_f64();
            if exposure_quote > self.max_exposure_in_quote {
                reasons.push(RiskReason::ExposureLimit {
                    side: Side::Buy,
                    exposure_quote,
                    max_exposure_in_quote: self.max_exposure_in_quote,
                });
            }
        }

        if let Some(ask) = ctx.target.ask {
            let projected_base = ctx.inventory.base - ask.quantity;
            let exposure_quote = projected_base * mid.as_f64();
            if exposure_quote < -self.max_exposure_in_quote {
                reasons.push(RiskReason::ExposureLimit {
                    side: Side::Sell,
                    exposure_quote,
                    max_exposure_in_quote: self.max_exposure_in_quote,
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
