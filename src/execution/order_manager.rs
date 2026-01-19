use anyhow::Result;
use std::time::Instant;

use crate::{
    execution::{
        order_action::{OrderAction, Side},
        order_report::OrderReport,
        order_side_manager::{OrderSideManager, SideInputs},
        types::OrderSideState,
    },
    types::{instrument::Instrument, quote_target::QuoteTarget},
};

#[derive(Debug)]
pub struct OrderManager {
    bid_side: OrderSideManager,
    ask_side: OrderSideManager,
}

impl Default for OrderManager {
    fn default() -> Self {
        Self {
            bid_side: OrderSideManager::for_side(Side::Buy),
            ask_side: OrderSideManager::for_side(Side::Sell),
        }
    }
}

impl OrderManager {
    pub fn on_report(&mut self, report: OrderReport) {
        self.bid_side.on_report(&report);
        self.ask_side.on_report(&report);
    }

    pub fn has_live_orders(&self) -> bool {
        matches!(self.bid_side.state(), OrderSideState::Live { .. })
            || matches!(self.ask_side.state(), OrderSideState::Live { .. })
    }

    pub fn has_inflight_actions(&self) -> bool {
        self.bid_side.has_inflight_actions() || self.ask_side.has_inflight_actions()
    }

    pub async fn actions_for_target(
        &mut self,
        instrument: &Instrument,
        target: &QuoteTarget,
        now: Instant,
    ) -> Result<Vec<OrderAction>> {
        let price_tick = instrument.trading_rules().price_tick;

        let mut actions = Vec::new();

        let bid_actions = self
            .bid_side
            .actions_for_target(SideInputs::new(instrument, now, price_tick, target.bid));

        let ask_actions = self
            .ask_side
            .actions_for_target(SideInputs::new(instrument, now, price_tick, target.ask));

        actions.extend(bid_actions);
        actions.extend(ask_actions);

        Ok(actions)
    }
}
