use crate::{
    market::market_state::MarketState,
    signals::signal_state::SignalState,
    strategy::{
        Strategy,
        instrument_context::{InstrumentContext, WithContext},
        strategy_helpers::StrategyHelpers,
    },
    types::{
        instrument::Instrument, inventory::Inventory, quote::Quote, quote_target::QuoteTarget,
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
}

impl MakerOnlyMeanReversionStrategy {
    pub fn for_instrument(instrument: &Instrument) -> Self {
        Self {
            ctx: InstrumentContext::new(instrument),
            entry_threshold_bps: 8.0,
            max_exposure_quote: 200.0,
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
    ) -> Option<QuoteTarget> {
        let (best_bid, best_ask) = Self::best_bid_ask(market_state)?;
        let rules = self.ctx().rules();
        let tick = self.ctx().tick();

        // Mid
        let mid = market_state.mid_price()?.as_f64();

        // EMA anchor (already f64)
        let ema = signal_state.ema_mid()?;

        // Deviation from EMA in bps (positive => mid above EMA)
        let dev_bps = (mid - ema) / ema.max(1e-12) * 10_000.0;

        // No edge → no quote
        if dev_bps.abs() < self.entry_threshold_bps {
            return None;
        }

        // Inventory exposure gate (quote currency at EMA)
        let exposure_quote = inventory.base * ema;
        let too_long = exposure_quote > self.max_exposure_quote;
        let too_short = exposure_quote < -self.max_exposure_quote;

        // Size from notional (use helpers / ctx)
        let quantity = self.size_from_notional(ema)?;
        if quantity <= 0.0 {
            return None;
        }

        let spread = best_ask - best_bid;
        let can_improve = self.improve_if_possible && spread >= 2.0 * tick;

        // One-sided mean reversion
        if dev_bps > 0.0 {
            // Price stretched UP → SELL (place ask)
            if too_short {
                return None;
            }

            let mut desired_ask = best_ask;
            if can_improve {
                desired_ask = best_ask - tick;
            }
            desired_ask = self.clamp_post_only_ask(desired_ask, best_bid);

            Some(QuoteTarget {
                bid: None,
                ask: Some(Quote {
                    price: rules.round_price_to_tick(desired_ask),
                    quantity,
                }),
            })
        } else {
            // Price stretched DOWN → BUY (place bid)
            if too_long {
                return None;
            }

            let mut desired_bid = best_bid;
            if can_improve {
                desired_bid = best_bid + tick;
            }
            desired_bid = self.clamp_post_only_bid(desired_bid, best_ask);

            Some(QuoteTarget {
                bid: Some(Quote {
                    price: rules.round_price_to_tick(desired_bid),
                    quantity,
                }),
                ask: None,
            })
        }
    }
}
