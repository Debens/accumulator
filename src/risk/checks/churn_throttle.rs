use std::time::{Duration, Instant};

use crate::risk::context::RiskContext;
use crate::risk::decision::RiskReason;
use crate::risk::engine::RiskCheck;

#[derive(Debug)]
pub struct ChurnThrottleCheck {
    pub min_update_interval: Duration,
    last_bid_update: Option<Instant>,
    last_ask_update: Option<Instant>,
    last_bid_price: Option<f64>,
    last_ask_price: Option<f64>,
}

impl ChurnThrottleCheck {
    pub fn new(min_update_interval: Duration) -> Self {
        Self {
            min_update_interval,
            last_bid_update: None,
            last_ask_update: None,
            last_bid_price: None,
            last_ask_price: None,
        }
    }

    fn is_too_soon(now: Instant, last_update: Option<Instant>, min_interval: Duration) -> bool {
        match last_update {
            None => false,
            Some(previous) => now.duration_since(previous) < min_interval,
        }
    }

    fn moved_enough(previous: Option<f64>, next: f64, tick: f64) -> bool {
        match previous {
            None => true,
            Some(value) => {
                let eps = (tick.abs() * 0.5).max(1e-12);

                (next - value).abs() >= eps
            }
        }
    }
}

impl RiskCheck for ChurnThrottleCheck {
    fn name(&self) -> &'static str {
        "ChurnThrottleCheck"
    }

    fn evaluate(&mut self, context: &RiskContext) -> Result<(), Vec<RiskReason>> {
        let now = context.now;
        let tick = context.instrument.trading_rules().price_tick;

        let mut reasons: Vec<RiskReason> = Vec::new();

        if let Some(bid) = &context.target.bid {
            let bid_price = bid.price.as_f64();
            let bid_changed = Self::moved_enough(self.last_bid_price, bid_price, tick);

            if bid_changed && Self::is_too_soon(now, self.last_bid_update, self.min_update_interval)
            {
                reasons.push(RiskReason::ChurnThrottleBid);
            } else if bid_changed {
                self.last_bid_update = Some(now);
                self.last_bid_price = Some(bid_price);
            }
        }

        if let Some(ask) = &context.target.ask {
            let ask_price = ask.price.as_f64();
            let ask_changed = Self::moved_enough(self.last_ask_price, ask_price, tick);

            if ask_changed && Self::is_too_soon(now, self.last_ask_update, self.min_update_interval)
            {
                reasons.push(RiskReason::ChurnThrottleAsk);
            } else if ask_changed {
                self.last_ask_update = Some(now);
                self.last_ask_price = Some(ask_price);
            }
        }

        if reasons.is_empty() {
            Ok(())
        } else {
            Err(reasons)
        }
    }
}
