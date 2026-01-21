use std::cell::Cell;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Regime {
    MeanReversion,
    TrendFollowing,
}

/// Regime switcher:
/// - Use slow EMA trend strength to choose between
///   mean reversion (weak trend) and trend following (strong trend)
#[derive(Debug, Clone)]
pub struct RegimeSwitchStrategy {
    ctx: InstrumentContext,
    mean_reversion: MakerOnlyMeanReversionStrategy,
    trend_following: MakerOnlyTrendFollowingStrategy,
    current_regime: Cell<Regime>,
    ticks_in_regime: Cell<u64>,
    /// Minimum ticks to stay in a regime before switching again.
    pub min_regime_ticks: u64,
    /// Minimum trend strength (in ticks) to switch into trend following.
    pub trend_enter_threshold_ticks: f64,
    /// Trend strength (in ticks) to switch back to mean reversion.
    pub trend_exit_threshold_ticks: f64,
    /// Minimum fast/slow EMA separation required to consider trend following.
    pub trend_slope_threshold_ticks: f64,
    /// Additional trend strength requirement relative to recent volatility.
    pub trend_strength_multiplier: f64,
}

impl RegimeSwitchStrategy {
    pub fn for_instrument(instrument: &Instrument) -> Self {
        Self {
            ctx: InstrumentContext::new(instrument),
            mean_reversion: MakerOnlyMeanReversionStrategy::for_instrument(instrument),
            trend_following: MakerOnlyTrendFollowingStrategy::for_instrument(instrument),
            current_regime: Cell::new(Regime::MeanReversion),
            ticks_in_regime: Cell::new(0),
            min_regime_ticks: 20,
            trend_enter_threshold_ticks: 6.0,
            trend_exit_threshold_ticks: 4.0,
            trend_slope_threshold_ticks: 2.0,
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

        let ema_fast = signal_state.ema_mid().ok_or(NoQuoteReason::MissingEma)?;
        let ema_slow = signal_state
            .ema_mid_slow()
            .ok_or(NoQuoteReason::MissingSlowEma)?;
        let trend_abs = (mid - ema_slow).abs();
        let slope_abs = (ema_fast - ema_slow).abs();
        let tick = self.ctx().tick();
        let slope_threshold_abs = self.trend_slope_threshold_ticks * tick;
        let base_enter_threshold = self.trend_enter_threshold_ticks * tick;
        let base_exit_threshold = self.trend_exit_threshold_ticks * tick;
        let vol_threshold = signal_state
            .volatility_mid()
            .map(|vol| vol * self.trend_strength_multiplier)
            .unwrap_or(0.0);
        let enter_threshold = base_enter_threshold + vol_threshold;
        let exit_threshold = base_exit_threshold + vol_threshold;

        let current_regime = self.current_regime.get();
        let ticks_in_regime = self.ticks_in_regime.get().saturating_add(1);
        self.ticks_in_regime.set(ticks_in_regime);

        let wants_trend = trend_abs >= enter_threshold && slope_abs >= slope_threshold_abs;
        let wants_mr = trend_abs <= exit_threshold || slope_abs < slope_threshold_abs;

        let mut next_regime = current_regime;
        if current_regime == Regime::MeanReversion && wants_trend {
            if ticks_in_regime >= self.min_regime_ticks {
                next_regime = Regime::TrendFollowing;
            }
        } else if current_regime == Regime::TrendFollowing && wants_mr {
            if ticks_in_regime >= self.min_regime_ticks {
                next_regime = Regime::MeanReversion;
            }
        }

        if next_regime != current_regime {
            tracing::info!(current_regime = ?current_regime, next_regime = ?next_regime, "regime switched");

            self.current_regime.set(next_regime);
            self.ticks_in_regime.set(0);
        }

        match self.current_regime.get() {
            Regime::TrendFollowing => {
                self.trend_following
                    .compute_target(market_state, signal_state, inventory)
            }
            Regime::MeanReversion => {
                self.mean_reversion
                    .compute_target(market_state, signal_state, inventory)
            }
        }
    }
}
