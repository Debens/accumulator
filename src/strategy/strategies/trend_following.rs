use crate::{
    market::market_state::MarketState,
    signals::signal_state::SignalState,
    strategy::{
        instrument_context::{InstrumentContext, WithContext},
        strategy::Strategy,
        strategy_helpers::StrategyHelpers,
    },
    types::{
        instrument::Instrument,
        inventory::Inventory,
        quote::Quote,
        quote_target::{NoQuoteReason, QuoteTarget},
    },
};

/// Maker-only Trend Following (one-sided):
/// - Use slow EMA(mid) to detect macro trend
/// - Trade in trend direction, prefer pullbacks vs fast EMA
/// - Place ONE post-only order near the touch
#[derive(Debug, Clone)]
pub struct MakerOnlyTrendFollowingStrategy {
    ctx: InstrumentContext,

    /// Maximum absolute exposure in quote currency
    pub max_exposure_in_quote: f64,

    /// Improve by 1 tick if spread allows
    pub improve_if_possible: bool,

    /// Minimum deviation from slow EMA required to trade (in ticks)
    pub entry_threshold_ticks: f64,

    /// Extra entry threshold scaled by recent volatility (price units per tick)
    pub volatility_entry_multiplier: f64,

    /// Minimum fast/slow EMA separation required to trade (in ticks)
    pub slope_threshold_ticks: f64,

    /// Require price to be on the "pullback" side of fast EMA
    pub require_pullback: bool,

    /// Allow a band around fast EMA before treating it as "no pullback" (in ticks)
    pub pullback_tolerance_ticks: f64,
}

impl MakerOnlyTrendFollowingStrategy {
    pub fn for_instrument(instrument: &Instrument) -> Self {
        let max_exposure_in_quote = instrument.trading_rules().max_exposure_in_quote;
        Self {
            ctx: InstrumentContext::new(instrument),
            max_exposure_in_quote,
            entry_threshold_ticks: 3.0,
            volatility_entry_multiplier: 1.0,
            slope_threshold_ticks: 2.0,
            improve_if_possible: true,
            require_pullback: true,
            pullback_tolerance_ticks: 2.0,
        }
    }
}

impl WithContext for MakerOnlyTrendFollowingStrategy {
    fn ctx(&self) -> &InstrumentContext {
        &self.ctx
    }
}

impl Strategy for MakerOnlyTrendFollowingStrategy {
    fn compute_target(
        &self,
        market_state: &MarketState,
        signal_state: &SignalState,
        _inventory: Inventory,
    ) -> Result<QuoteTarget, NoQuoteReason> {
        let (best_bid, best_ask) =
            Self::best_bid_ask(market_state).ok_or(NoQuoteReason::MissingTopOfBook)?;

        let rules = self.ctx().rules();
        let tick = self.ctx().tick();

        let mid = market_state
            .mid_price()
            .map(|p| p.as_f64())
            .ok_or(NoQuoteReason::MissingMid)?;

        let ema_fast = signal_state.ema_mid().ok_or(NoQuoteReason::MissingEma)?;
        let ema_slow = signal_state.ema_mid_slow().unwrap_or(ema_fast);

        let trend = mid - ema_slow;
        let trend_abs = trend.abs();
        let vol_threshold = signal_state
            .volatility_mid()
            .unwrap_or(0.0)
            * self.volatility_entry_multiplier;
        let threshold_abs = self.entry_threshold_ticks * tick + vol_threshold;
        if trend_abs < threshold_abs {
            return Err(NoQuoteReason::BelowEntryThreshold {
                deviation_ticks: trend_abs / tick,
                threshold_ticks: threshold_abs / tick,
            });
        }

        let slope_abs = (ema_fast - ema_slow).abs();
        let slope_threshold_abs = self.slope_threshold_ticks * tick;
        if slope_threshold_abs > 0.0 && slope_abs < slope_threshold_abs {
            return Err(NoQuoteReason::BelowTrendSlopeThreshold {
                slope_ticks: slope_abs / tick,
                threshold_ticks: self.slope_threshold_ticks,
            });
        }

        let quantity = self
            .size_from_notional(ema_fast)
            .ok_or(NoQuoteReason::InvalidQuantity)?;
        if quantity <= 0.0 {
            return Err(NoQuoteReason::InvalidQuantity);
        }

        let spread = best_ask - best_bid;
        let can_improve = self.improve_if_possible && spread >= 2.0 * tick;
        let pullback_tolerance = self.pullback_tolerance_ticks * tick;

        if trend > 0.0 {
            // Uptrend → BUY on pullback
            if self.require_pullback && mid > ema_fast + pullback_tolerance {
                return Err(NoQuoteReason::PullbackNotMet);
            }

            let mut desired_bid = best_bid;
            if can_improve {
                desired_bid = best_bid + tick;
            }
            desired_bid = self.clamp_bid(desired_bid, best_ask);

            if desired_bid > best_ask - tick {
                return Err(NoQuoteReason::WouldCrossPostOnly);
            }

            let bid_price = rules.round_price_to_tick(desired_bid);
            if bid_price.as_f64() > best_ask - tick {
                return Err(NoQuoteReason::WouldCrossPostOnly);
            }

            Ok(QuoteTarget {
                bid: Some(Quote {
                    price: bid_price,
                    quantity,
                }),
                ask: None,
            })
        } else {
            // Downtrend → SELL on pullback
            if self.require_pullback && mid < ema_fast - pullback_tolerance {
                return Err(NoQuoteReason::PullbackNotMet);
            }

            let mut desired_ask = best_ask;
            if can_improve {
                desired_ask = best_ask - tick;
            }
            desired_ask = self.clamp_ask(desired_ask, best_bid);

            if desired_ask < best_bid + tick {
                return Err(NoQuoteReason::WouldCrossPostOnly);
            }

            let ask_price = rules.round_price_to_tick(desired_ask);
            if ask_price.as_f64() < best_bid + tick {
                return Err(NoQuoteReason::WouldCrossPostOnly);
            }

            Ok(QuoteTarget {
                bid: None,
                ask: Some(Quote {
                    price: ask_price,
                    quantity,
                }),
            })
        }
    }
}
