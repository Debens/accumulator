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

/// Maker-only Mean Reversion (one-sided):
/// - Use EMA(mid) as anchor
/// - Only trade when mid deviates sufficiently from EMA
/// - Place ONE post-only order near the touch
#[derive(Debug, Clone)]
pub struct MakerOnlyMeanReversionStrategy {
    ctx: InstrumentContext,

    /// Maximum absolute exposure in quote currency
    pub max_exposure_in_quote: f64,

    /// Improve by 1 tick if spread allows
    pub improve_if_possible: bool,

    /// Minimum deviation from EMA required to trade (in ticks)
    pub entry_threshold_ticks: f64,

    /// Trend filter deadband around slow EMA (in ticks)
    pub trend_filter_ticks: f64,

    /// Multiplier for entry threshold when trading against the slow-EMA trend
    pub counter_trend_multiplier: f64,

    /// Additional threshold multiplier (0..n) based on exposure in the trade direction
    pub inventory_penalty: f64,
}

impl MakerOnlyMeanReversionStrategy {
    pub fn for_instrument(instrument: &Instrument) -> Self {
        let max_exposure_in_quote = instrument.trading_rules().max_exposure_in_quote;
        Self {
            ctx: InstrumentContext::new(instrument),
            max_exposure_in_quote,
            entry_threshold_ticks: 3.0,
            improve_if_possible: true,
            trend_filter_ticks: 2.0,
            counter_trend_multiplier: 2.0,
            inventory_penalty: 1.0,
        }
    }
}

impl WithContext for MakerOnlyMeanReversionStrategy {
    fn ctx(&self) -> &InstrumentContext {
        &self.ctx
    }
}

impl Strategy for MakerOnlyMeanReversionStrategy {
    fn compute_target(
        &self,
        market_state: &MarketState,
        signal_state: &SignalState,
        inventory: Inventory,
    ) -> Result<QuoteTarget, NoQuoteReason> {
        let (best_bid, best_ask) =
            Self::best_bid_ask(market_state).ok_or(NoQuoteReason::MissingTopOfBook)?;

        let rules = self.ctx().rules();
        let tick = self.ctx().tick();

        let mid = market_state
            .mid_price()
            .map(|p| p.as_f64())
            .ok_or(NoQuoteReason::MissingMid)?;

        let ema = signal_state.ema_mid().ok_or(NoQuoteReason::MissingEma)?;
        let ema_slow = signal_state.ema_mid_slow().unwrap_or(ema);

        let deviation = mid - ema;
        let deviation_abs = deviation.abs();
        let trend = mid - ema_slow;
        let trend_deadband = self.trend_filter_ticks * tick;

        let quantity = self
            .size_from_notional(ema)
            .ok_or(NoQuoteReason::InvalidQuantity)?;
        if quantity <= 0.0 {
            return Err(NoQuoteReason::InvalidQuantity);
        }

        let spread = best_ask - best_bid;
        let can_improve = self.improve_if_possible && spread >= 2.0 * tick;
        let exposure_quote = inventory.base * mid;
        let exposure_norm =
            (exposure_quote / self.max_exposure_in_quote.max(1e-12)).clamp(-1.0, 1.0);

        if deviation > 0.0 {
            let is_counter_trend = trend > trend_deadband;
            let mut threshold_ticks = self.entry_threshold_ticks;
            if is_counter_trend {
                threshold_ticks *= self.counter_trend_multiplier;
            }
            if exposure_norm < 0.0 {
                threshold_ticks *= 1.0 + exposure_norm.abs() * self.inventory_penalty;
            }
            let threshold_abs = threshold_ticks * tick;
            if deviation_abs < threshold_abs {
                return Err(NoQuoteReason::BelowEntryThreshold {
                    deviation_ticks: deviation_abs / tick,
                    threshold_ticks,
                });
            }

            // Price stretched UP → SELL (place ask)
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
        } else {
            let is_counter_trend = trend < -trend_deadband;
            let mut threshold_ticks = self.entry_threshold_ticks;
            if is_counter_trend {
                threshold_ticks *= self.counter_trend_multiplier;
            }
            if exposure_norm > 0.0 {
                threshold_ticks *= 1.0 + exposure_norm.abs() * self.inventory_penalty;
            }
            let threshold_abs = threshold_ticks * tick;
            if deviation_abs < threshold_abs {
                return Err(NoQuoteReason::BelowEntryThreshold {
                    deviation_ticks: deviation_abs / tick,
                    threshold_ticks,
                });
            }

            // Price stretched DOWN → BUY (place bid)
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
        }
    }
}
