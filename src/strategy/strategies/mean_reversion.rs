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

    /// Minimum deviation from EMA required to trade (bps)
    pub entry_threshold_bps: f64,

    /// Maximum absolute exposure in quote currency
    pub max_exposure_quote: f64,

    /// Improve by 1 tick if spread allows
    pub improve_if_possible: bool,

    /// Minimum deviation from EMA required to trade (in ticks)
    pub entry_threshold_ticks: f64,
}

impl MakerOnlyMeanReversionStrategy {
    pub fn for_instrument(instrument: &Instrument) -> Self {
        Self {
            ctx: InstrumentContext::new(instrument),
            entry_threshold_bps: 8.0,
            max_exposure_quote: 200.0,
            entry_threshold_ticks: 3.0,
            improve_if_possible: true,
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

        let deviation = mid - ema;
        let deviation_abs = deviation.abs();
        let threshold_abs = self.entry_threshold_ticks * tick;

        if deviation_abs < threshold_abs {
            return Err(NoQuoteReason::BelowEntryThreshold {
                deviation_ticks: deviation_abs / tick,
                threshold_bps: self.entry_threshold_bps,
            });
        }

        // Inventory exposure gate (quote currency at EMA)
        let exposure_quote = inventory.base * ema;
        if exposure_quote > self.max_exposure_quote {
            return Err(NoQuoteReason::TooLongExposure {
                exposure_quote,
                max_exposure_quote: self.max_exposure_quote,
            });
        }
        if exposure_quote < -self.max_exposure_quote {
            return Err(NoQuoteReason::TooShortExposure {
                exposure_quote,
                max_exposure_quote: self.max_exposure_quote,
            });
        }

        let quantity = self
            .size_from_notional(ema)
            .ok_or(NoQuoteReason::InvalidQuantity)?;
        if quantity <= 0.0 {
            return Err(NoQuoteReason::InvalidQuantity);
        }

        let spread = best_ask - best_bid;
        let can_improve = self.improve_if_possible && spread >= 2.0 * tick;

        // One-sided mean reversion
        if deviation > 0.0 {
            // Price stretched UP → SELL (place ask)
            let mut desired_ask = best_ask;
            if can_improve {
                desired_ask = best_ask - tick;
            }
            desired_ask = self.clamp_post_only_ask(desired_ask, best_bid);

            // Optional sanity check: if book/tick is weird, post-only clamp may still be invalid
            if desired_ask < best_bid + tick {
                return Err(NoQuoteReason::WouldCrossPostOnly);
            }

            Ok(QuoteTarget {
                bid: None,
                ask: Some(Quote {
                    price: rules.round_price_to_tick(desired_ask),
                    quantity,
                }),
            })
        } else {
            // Price stretched DOWN → BUY (place bid)
            let mut desired_bid = best_bid;
            if can_improve {
                desired_bid = best_bid + tick;
            }
            desired_bid = self.clamp_post_only_bid(desired_bid, best_ask);

            if desired_bid > best_ask - tick {
                return Err(NoQuoteReason::WouldCrossPostOnly);
            }

            Ok(QuoteTarget {
                bid: Some(Quote {
                    price: rules.round_price_to_tick(desired_bid),
                    quantity,
                }),
                ask: None,
            })
        }
    }
}
