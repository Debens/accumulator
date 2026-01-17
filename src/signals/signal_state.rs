use std::time::Instant;

use crate::market::market_state::MarketState;
use crate::signals::ema::Ema;

#[derive(Debug)]
pub struct SignalState {
    ema_mid: Ema,
    last_ema_value: Option<f64>,
}

impl SignalState {
    pub fn new(ema_tau_seconds: f64) -> Self {
        Self {
            ema_mid: Ema::new(ema_tau_seconds),
            last_ema_value: None,
        }
    }

    pub fn update(&mut self, market_state: &MarketState, now: Instant) {
        if let Some(mid) = market_state.mid_price() {
            let ema_value = self.ema_mid.update(now, mid.as_f64());
            self.last_ema_value = Some(ema_value);
        }
    }

    pub fn ema_mid(&self) -> Option<f64> {
        self.last_ema_value
    }
}
