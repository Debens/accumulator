use std::time::Instant;

use crate::{
    execution::order_manager::OrderManager, market::market_state::MarketState,
    signals::signal_state::SignalState, types::instrument::Instrument,
};

pub struct ScheduleContext<'a> {
    pub now: Instant,
    pub instrument: &'a Instrument,
    pub market_state: &'a MarketState,
    pub signal_state: &'a SignalState,
    pub order_manager: &'a OrderManager,
}
