use std::time::Instant;

use crate::market::market_state::MarketState;
use crate::types::inventory::Inventory;
use crate::types::instrument::Instrument;
use crate::types::quote_target::QuoteTarget;

#[derive(Debug)]
pub struct RiskContext<'a> {
    pub instrument: &'a Instrument,
    pub market_state: &'a MarketState,
    pub target: &'a QuoteTarget,
    pub inventory: Inventory,
    pub now: Instant,
}
