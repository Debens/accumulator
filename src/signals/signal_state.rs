use std::time::{Duration, Instant};

use crate::market::market_state::MarketState;
use crate::signals::ema::Ema;

#[derive(Debug)]
pub struct SignalState {
    ema_mid: Ema,
    ema_mid_slow: Ema,
    ema_abs_mid_change: Ema,
    last_ema_value: Option<f64>,
    last_ema_slow_value: Option<f64>,
    last_volatility: Option<f64>,
    last_mid: Option<f64>,
    last_update: Option<Instant>,
    min_update_interval: Duration,
}

impl SignalState {
    pub fn new(fast_tau_seconds: f64, slow_tau_seconds: f64, vol_tau_seconds: f64) -> Self {
        Self {
            ema_mid: Ema::new(fast_tau_seconds),
            ema_mid_slow: Ema::new(slow_tau_seconds),
            ema_abs_mid_change: Ema::new(vol_tau_seconds),
            last_ema_value: None,
            last_ema_slow_value: None,
            last_volatility: None,
            last_mid: None,
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
            let mid_value = mid.as_f64();
            if let Some(last_mid) = self.last_mid {
                let abs_change = (mid_value - last_mid).abs();
                let vol = self.ema_abs_mid_change.update(now, abs_change);
                self.last_volatility = Some(vol);
            }

            let ema_fast = self.ema_mid.update(now, mid_value);
            let ema_slow = self.ema_mid_slow.update(now, mid_value);
            self.last_ema_value = Some(ema_fast);
            self.last_ema_slow_value = Some(ema_slow);
            self.last_mid = Some(mid_value);
            self.last_update = Some(now);
        }
    }

    pub fn ema_mid(&self) -> Option<f64> {
        self.last_ema_value
    }

    pub fn ema_mid_slow(&self) -> Option<f64> {
        self.last_ema_slow_value
    }

    pub fn volatility_mid(&self) -> Option<f64> {
        self.last_volatility
    }
}
