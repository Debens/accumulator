use crate::{
    market::market_state::MarketState, signals::signal_state::SignalState,
    strategy::instrument_context::WithContext,
};

pub trait StrategyHelpers: WithContext {
    fn best_bid_ask(market_state: &MarketState) -> Option<(f64, f64)> {
        Some((
            market_state.best_bid()?.as_f64(),
            market_state.best_ask()?.as_f64(),
        ))
    }

    fn fair_price(market_state: &MarketState, signal_state: &SignalState) -> Option<f64> {
        signal_state
            .ema_mid()
            .or_else(|| market_state.mid_price().map(|p| p.as_f64()))
    }

    fn size_from_notional(&self, price: f64) -> Option<f64> {
        let rules = self.ctx().rules();
        let q = rules.quantity_from_notional(self.ctx().max_order_notional(), price);
        (q > 0.0).then_some(q)
    }

    fn clamp_bid(&self, bid: f64, best_ask: f64) -> f64 {
        bid.min(best_ask - self.ctx().tick())
    }

    fn clamp_ask(&self, ask: f64, best_bid: f64) -> f64 {
        ask.max(best_bid + self.ctx().tick())
    }
}

impl<T: WithContext> StrategyHelpers for T {}
