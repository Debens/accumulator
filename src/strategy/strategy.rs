use crate::{
    market::market_state::MarketState,
    signals::signal_state::SignalState,
    strategy::instrument_context::WithContext,
    types::{
        inventory::Inventory,
        quote_target::{NoQuoteReason, QuoteTarget},
    },
};

pub trait Strategy: WithContext {
    fn compute_target(
        &self,
        market_state: &MarketState,
        signal_state: &SignalState,
        inventory: Inventory,
    ) -> Result<QuoteTarget, NoQuoteReason>;
}
