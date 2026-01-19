use std::time::{Duration, Instant};

use crate::market::market_state::MarketState;
use crate::signals::ema::Ema;

#[derive(Debug)]
pub struct SignalState {
    ema_mid: Ema,
    last_ema_value: Option<f64>,
    last_update: Option<Instant>,
    min_update_interval: Duration,
}

impl SignalState {
    pub fn new(ema_tau_seconds: f64) -> Self {
        Self {
            ema_mid: Ema::new(ema_tau_seconds),
            last_ema_value: None,
            last_update: None,
            min_update_interval: Duration::from_millis(350),
        }
    }

    pub fn update(&mut self, market_state: &MarketState, now: Instant) {
        if let Some(last) = self.last_update {
            if now.duration_since(last) < self.min_update_interval {
                return;
            }
        }

        if let Some(mid) = market_state.mid_price() {
            let ema_value = self.ema_mid.update(now, mid.as_f64());
            self.last_ema_value = Some(ema_value);
            self.last_update = Some(now);
        }
    }

    pub fn ema_mid(&self) -> Option<f64> {
        self.last_ema_value
    }
}
