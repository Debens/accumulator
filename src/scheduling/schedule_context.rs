use std::time::Instant;

use crate::types::instrument::Instrument;
use crate::{execution::order_manager::OrderManager, market::market_state::MarketState};

pub struct ScheduleContext<'a> {
    pub now: Instant,
    pub instrument: &'a Instrument,
    pub market_state: &'a MarketState,
    pub order_manager: &'a OrderManager,
}
