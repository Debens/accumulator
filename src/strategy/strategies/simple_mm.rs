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
    ) -> Option<QuoteTarget> {
        let (best_bid, best_ask) = Self::best_bid_ask(market_state)?;
        let rules = self.ctx().rules();
        let tick = self.ctx().tick();

        // Fair price: EMA(mid) preferred, fallback to raw mid.
        let fair = Self::fair_price(market_state, signal_state)?;

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
        let order_quantity = self.size_from_notional(skewed_fair)?;
        if order_quantity <= 0.0 {
            return None;
        }

        // ----- one-sided quoting if exposure is too large -----
        let too_long = exposure_quote > self.max_exposure_quote;
        let too_short = exposure_quote < -self.max_exposure_quote;

        // ----- price selection: quote at/near the touch -----
        let spread = best_ask - best_bid;
        let can_improve = spread >= 2.0 * tick;

        let mid = 0.5 * (best_bid + best_ask);
        let fair_bias = (skewed_fair - mid).signum(); // -1 sell bias, +1 buy bias, 0 neutral

        // Desired bid:
        // - default join best_bid
        // - if buy-biased and can improve: best_bid + 1 tick
        // - always remain post-only: bid <= best_ask - tick
        let mut desired_bid = best_bid;
        if !too_long && fair_bias > 0.0 && can_improve {
            desired_bid = best_bid + tick;
        }
        desired_bid = self.clamp_post_only_bid(desired_bid, best_ask);

        // Desired ask:
        // - default join best_ask
        // - if sell-biased and can improve: best_ask - 1 tick
        // - always remain post-only: ask >= best_bid + tick
        let mut desired_ask = best_ask;
        if !too_short && fair_bias < 0.0 && can_improve {
            desired_ask = best_ask - tick;
        }
        desired_ask = self.clamp_post_only_ask(desired_ask, best_bid);

        // Optional: enforce a minimum half-spread away from skewed fair *only if it doesn't make you uncompetitive*.
        // Example: if min_half_spread is huge (0.10) but touch spread is 0.01, we ignore it.
        let half_spread_floor = self.ctx().min_half_spread();
        let bid_floor_from_fair = skewed_fair - half_spread_floor;
        let ask_floor_from_fair = skewed_fair + half_spread_floor;

        // Only apply the floor if it doesn't move us *away* from the touch.
        // (Bid: raising it toward fair is ok if still post-only; Ask: lowering it toward fair is ok if still post-only.)
        desired_bid = desired_bid.max(bid_floor_from_fair);
        desired_bid = self.clamp_post_only_bid(desired_bid, best_ask);

        desired_ask = desired_ask.min(ask_floor_from_fair);
        desired_ask = self.clamp_post_only_ask(desired_ask, best_bid);

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
            return None;
        }

        Some(QuoteTarget { bid, ask })
    }
}
