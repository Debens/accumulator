use crate::{
    market::market_state::MarketState,
    signals::signal_state::SignalState,
    strategy::{
        instrument_context::{InstrumentContext, WithContext},
        strategy::Strategy,
    },
    types::{
        instrument::Instrument,
        inventory::Inventory,
        quote_target::{NoQuoteReason, QuoteTarget},
    },
};

use super::{
    mean_reversion::MakerOnlyMeanReversionStrategy,
    trend_following::MakerOnlyTrendFollowingStrategy,
};

/// Regime switcher:
/// - Use slow EMA trend strength to choose between
///   mean reversion (weak trend) and trend following (strong trend)
#[derive(Debug, Clone)]
pub struct RegimeSwitchStrategy {
    ctx: InstrumentContext,
    mean_reversion: MakerOnlyMeanReversionStrategy,
    trend_following: MakerOnlyTrendFollowingStrategy,
    /// Minimum trend strength (in ticks) to switch into trend following.
    pub trend_threshold_ticks: f64,
    /// Additional trend strength requirement relative to recent volatility.
    pub trend_strength_multiplier: f64,
}

impl RegimeSwitchStrategy {
    pub fn for_instrument(instrument: &Instrument) -> Self {
        Self {
            ctx: InstrumentContext::new(instrument),
            mean_reversion: MakerOnlyMeanReversionStrategy::for_instrument(instrument),
            trend_following: MakerOnlyTrendFollowingStrategy::for_instrument(instrument),
            trend_threshold_ticks: 6.0,
            trend_strength_multiplier: 2.5,
        }
    }
}

impl WithContext for RegimeSwitchStrategy {
    fn ctx(&self) -> &InstrumentContext {
        &self.ctx
    }
}

impl Strategy for RegimeSwitchStrategy {
    fn compute_target(
        &self,
        market_state: &MarketState,
        signal_state: &SignalState,
        inventory: Inventory,
    ) -> Result<QuoteTarget, NoQuoteReason> {
        let mid = market_state
            .mid_price()
            .map(|p| p.as_f64())
            .ok_or(NoQuoteReason::MissingMid)?;

        let ema_slow = signal_state
            .ema_mid_slow()
            .ok_or(NoQuoteReason::MissingSlowEma)?;
        let trend_abs = (mid - ema_slow).abs();
        let base_threshold = self.trend_threshold_ticks * self.ctx().tick();
        let vol_threshold = signal_state
            .volatility_mid()
            .map(|vol| vol * self.trend_strength_multiplier)
            .unwrap_or(0.0);
        let threshold = base_threshold.max(vol_threshold);

        if trend_abs >= threshold {
            self.trend_following
                .compute_target(market_state, signal_state, inventory)
        } else {
            self.mean_reversion
                .compute_target(market_state, signal_state, inventory)
        }
    }
}
