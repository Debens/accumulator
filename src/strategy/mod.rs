pub mod instrument_context;
pub mod strategies;
pub mod strategy;
pub mod strategy_helpers;

use crate::{
    market::market_state::MarketState,
    signals::signal_state::SignalState,
    strategy::instrument_context::WithContext,
    types::{inventory::Inventory, quote_target::QuoteTarget},
};

pub trait Strategy: WithContext {
    fn compute_target(
        &self,
        market_state: &MarketState,
        signal_state: &SignalState,
        inventory: Inventory,
    ) -> Option<QuoteTarget>;
}
