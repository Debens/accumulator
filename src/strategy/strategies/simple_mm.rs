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

#[derive(Debug, Clone)]
pub struct SimpleMarketMakerStrategy {
    ctx: InstrumentContext,
    pub max_exposure_quote: f64,
    pub max_skew_bps: f64,
}

impl SimpleMarketMakerStrategy {
    pub fn new(instrument: &Instrument, max_exposure_quote: f64, max_skew_bps: f64) -> Self {
        Self {
            ctx: InstrumentContext::new(instrument),
            max_exposure_quote,
            max_skew_bps,
        }
    }

    pub fn for_instrument(instrument: &Instrument) -> Self {
        Self::new(instrument, 200.0, 10.0)
    }
}

impl WithContext for SimpleMarketMakerStrategy {
    fn ctx(&self) -> &InstrumentContext {
        &self.ctx
    }
}

impl Strategy for SimpleMarketMakerStrategy {
    fn compute_target(
        &self,
        market_state: &MarketState,
        signal_state: &SignalState,
        inventory: Inventory,
    ) -> Result<QuoteTarget, crate::types::quote_target::NoQuoteReason> {
        let (best_bid, best_ask) =
            Self::best_bid_ask(market_state).ok_or(NoQuoteReason::MissingTopOfBook)?;

        let rules = self.ctx().rules();
        let tick = self.ctx().tick();

        // Fair price: EMA(mid) preferred, fallback to raw mid.
        let fair =
            Self::fair_price(market_state, signal_state).ok_or(NoQuoteReason::MissingFairPrice)?;

        // ----- inventory-aware fair price -----
        let exposure_quote = inventory.base * fair;

        // Normalize exposure into [-1, 1] relative to max exposure cap.
        let denom = self.max_exposure_quote.max(1e-12);
        let norm = (exposure_quote / denom).clamp(-1.0, 1.0);

        // Positive exposure => skew fair downward to encourage sells.
        let skew_bps = norm * self.max_skew_bps;
        let skew = fair * (skew_bps / 10_000.0);
        let skewed_fair = fair - skew;

        // ----- size (quote currency notional cap) -----
        let order_quantity = self
            .size_from_notional(skewed_fair)
            .ok_or(NoQuoteReason::InvalidQuantity)?;
        if order_quantity <= 0.0 {
            return Err(NoQuoteReason::InvalidQuantity);
        }

        // ----- one-sided quoting if exposure is too large -----
        let too_long = exposure_quote > self.max_exposure_quote;
        let too_short = exposure_quote < -self.max_exposure_quote;

        // If you prefer hard-stop reasons (instead of just suppressing one side),
        // you can return these immediately:
        //
        // if too_long { return Err(NoQuoteReason::TooLongExposure { exposure_quote, max_exposure_quote: self.max_exposure_quote }); }
        // if too_short { return Err(NoQuoteReason::TooShortExposure { exposure_quote, max_exposure_quote: self.max_exposure_quote }); }

        // ----- price selection: quote at/near the touch -----
        let spread = best_ask - best_bid;
        let can_improve = spread >= 2.0 * tick;

        let mid = 0.5 * (best_bid + best_ask);
        let fair_bias = (skewed_fair - mid).signum(); // -1 sell bias, +1 buy bias, 0 neutral

        // Desired bid:
        let mut desired_bid = best_bid;
        if !too_long && fair_bias > 0.0 && can_improve {
            desired_bid = best_bid + tick;
        }
        desired_bid = self.clamp_post_only_bid(desired_bid, best_ask);

        // Desired ask:
        let mut desired_ask = best_ask;
        if !too_short && fair_bias < 0.0 && can_improve {
            desired_ask = best_ask - tick;
        }
        desired_ask = self.clamp_post_only_ask(desired_ask, best_bid);

        // Optional: enforce a minimum half-spread away from skewed fair *only if it doesn't make you uncompetitive*.
        let half_spread_floor = self.ctx().min_half_spread();
        let bid_floor_from_fair = skewed_fair - half_spread_floor;
        let ask_floor_from_fair = skewed_fair + half_spread_floor;

        desired_bid = desired_bid.max(bid_floor_from_fair);
        desired_bid = self.clamp_post_only_bid(desired_bid, best_ask);

        desired_ask = desired_ask.min(ask_floor_from_fair);
        desired_ask = self.clamp_post_only_ask(desired_ask, best_bid);

        // Sanity: if tick/book is weird, ensure post-only invariants still hold.
        if desired_bid > best_ask - tick || desired_ask < best_bid + tick {
            return Err(NoQuoteReason::WouldCrossPostOnly);
        }

        let bid_price = rules.round_price_to_tick(desired_bid);
        let ask_price = rules.round_price_to_tick(desired_ask);

        let bid = if too_long {
            None
        } else {
            Some(Quote {
                price: bid_price,
                quantity: order_quantity,
            })
        };

        let ask = if too_short {
            None
        } else {
            Some(Quote {
                price: ask_price,
                quantity: order_quantity,
            })
        };

        if bid.is_none() && ask.is_none() {
            // If you want a more specific reason, you can split this into:
            // - too_long && too_short (shouldn't happen)
            // - etc
            return Err(NoQuoteReason::BothSidesSuppressedByExposure);
        }

        Ok(QuoteTarget { bid, ask })
    }
}
