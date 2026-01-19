use std::time::{Duration, Instant};

use uuid::Uuid;

use crate::{
    execution::{
        order_action::{Order, OrderAction, OrderType, Side},
        order_report::OrderReport,
        types::{OrderSideState, SidePlan},
    },
    types::{instrument::Instrument, quote::Quote},
};

#[derive(Debug, Clone)]
pub struct SideInputs<'a> {
    instrument: &'a Instrument,
    now: Instant,
    price_tick: f64,
    target: Option<Quote>,
}

impl<'a> SideInputs<'a> {
    pub fn new(
        instrument: &'a Instrument,
        now: Instant,
        price_tick: f64,
        target: Option<Quote>,
    ) -> Self {
        Self {
            instrument,
            now,
            price_tick,
            target,
        }
    }
}

impl Default for OrderSideState {
    fn default() -> Self {
        OrderSideState::NoOrder
    }
}

#[derive(Debug, Clone)]
pub struct ReplacePolicy {
    replace_threshold_ticks: i64,
    min_lifetime: Duration,
}

impl Default for ReplacePolicy {
    fn default() -> Self {
        Self {
            replace_threshold_ticks: 3,
            min_lifetime: Duration::from_millis(500),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct OrderSideManager {
    side: Side,
    state: OrderSideState,
    last_update: Option<Instant>,
    policy: ReplacePolicy,
}

impl OrderSideManager {
    pub fn has_inflight_actions(&self) -> bool {
        match &self.state {
            OrderSideState::Placing { .. } => true,
            OrderSideState::Cancelling { .. } => true,
            OrderSideState::Live { .. } => false,
            OrderSideState::NoOrder => false,
        }
    }

    pub fn for_side(side: Side) -> Self {
        Self {
            side,
            ..Default::default()
        }
    }

    pub fn on_report(&mut self, report: &OrderReport) {
        match report {
            OrderReport::Placed {
                order_id,
                side,
                price,
                quantity,
                ..
            } if *side == self.side => {
                self.state = OrderSideState::Placing {
                    order_id: order_id.clone(),
                    requested: Quote {
                        price: *price,
                        quantity: *quantity,
                    },
                };
            }

            OrderReport::Accepted {
                order_id,
                side,
                price,
                quantity,
                ..
            } if *side == self.side => {
                self.state = OrderSideState::Live {
                    order_id: order_id.clone(),
                    resting: Quote {
                        price: *price,
                        quantity: *quantity,
                    },
                };
                self.last_update = Some(Instant::now());
            }

            OrderReport::Rejected { order_id, side, .. } if *side == self.side => {
                if self.matches_current_order(order_id) {
                    self.state = OrderSideState::NoOrder;
                    self.last_update = None;
                }
            }

            OrderReport::Cancel { order_id, side, .. } if *side == self.side => {
                if let OrderSideState::Live {
                    order_id: live_id,
                    resting,
                } = self.state.clone()
                {
                    if *order_id == live_id {
                        self.state = OrderSideState::Cancelling {
                            order_id: order_id.clone(),
                            resting,
                        };
                    }
                }
            }

            OrderReport::Cancelled { order_id, side, .. } if *side == self.side => {
                if self.matches_current_order(order_id) {
                    self.state = OrderSideState::NoOrder;
                    self.last_update = None;
                }
            }

            OrderReport::PartiallyFilled {
                order_id,
                side,
                price,
                quantity,
                ..
            } if *side == self.side => {
                if let OrderSideState::Live {
                    order_id: live_id,
                    resting,
                } = self.state.clone()
                {
                    if *order_id == live_id {
                        let remaining = (resting.quantity - *quantity).max(0.0);

                        self.state = OrderSideState::Live {
                            order_id: live_id,
                            resting: Quote {
                                price: resting.price,
                                quantity: remaining,
                            },
                        };

                        self.last_update = Some(Instant::now());

                        tracing::info!(
                            side = %self.side,
                            order_id = %order_id,
                            fill_price = %price,
                            fill_quantity = *quantity,
                            remaining_quantity = remaining,
                            "order partially filled"
                        );
                    }
                }
            }

            OrderReport::Filled {
                order_id,
                side,
                price,
                quantity,
                ..
            } if *side == self.side => {
                if self.matches_current_order(order_id) {
                    tracing::info!(
                        side = %self.side,
                        order_id = %order_id,
                        fill_price = %price,
                        fill_quantity = *quantity,
                        "order filled"
                    );

                    self.state = OrderSideState::NoOrder;
                    self.last_update = None;
                }
            }

            _ => {}
        }
    }

    fn matches_current_order(&self, order_id: &str) -> bool {
        match &self.state {
            OrderSideState::Placing { order_id: id, .. } => id == order_id,
            OrderSideState::Live { order_id: id, .. } => id == order_id,
            OrderSideState::Cancelling { order_id: id, .. } => id == order_id,
            OrderSideState::NoOrder => false,
        }
    }

    pub fn actions_for_target(&mut self, inputs: SideInputs<'_>) -> Vec<OrderAction> {
        let plan = self.plan(&inputs);
        let actions = self.get_actions(inputs.instrument, &plan);
        self.apply_optimistic(plan, inputs.now);
        actions
    }

    fn plan(&self, inputs: &SideInputs<'_>) -> SidePlan {
        use crate::execution::types::OrderSideState::*;
        use crate::execution::types::SidePlan::*;

        match (&self.state, inputs.target.clone()) {
            (NoOrder, None) => NoAction,
            (NoOrder, Some(desired)) => Place {
                order_id: generate_order_id(inputs.instrument, self.side),
                desired,
            },

            (Placing { .. }, _) => WaitForVenue,
            (Cancelling { .. }, _) => WaitForVenue,

            (Live { order_id, .. }, None) => Cancel {
                order_id: order_id.clone(),
            },

            (Live { order_id, resting }, Some(desired)) => {
                if self.is_stale(&resting, &desired, inputs.now, inputs.price_tick) {
                    Replace {
                        old_order_id: order_id.clone(),
                        new_order_id: generate_order_id(inputs.instrument, self.side),
                        desired,
                    }
                } else {
                    NoAction
                }
            }
        }
    }

    fn is_stale(&self, current: &Quote, desired: &Quote, now: Instant, price_tick: f64) -> bool {
        if let Some(last_update) = self.last_update {
            if now.duration_since(last_update) < self.policy.min_lifetime {
                return false;
            }
        }

        let current_ticks = price_to_ticks(current.price.as_f64(), price_tick);
        let desired_ticks = price_to_ticks(desired.price.as_f64(), price_tick);
        let diff_ticks = (current_ticks - desired_ticks).abs();

        let quantity_changed = (current.quantity - desired.quantity).abs() > 1e-12;
        if quantity_changed {
            tracing::info!(current = ?current, desired = ?desired, "quantity changed");

            return true;
        }

        let ticks_threshold_triggered = diff_ticks >= self.policy.replace_threshold_ticks;
        if ticks_threshold_triggered {
            tracing::info!(current = ?current, desired = ?desired, "ticks threshold triggered");

            return true;
        }

        false
    }

    fn get_actions(&self, instrument: &Instrument, plan: &SidePlan) -> Vec<OrderAction> {
        use crate::execution::types::SidePlan::*;

        let mut actions = Vec::new();

        match plan {
            NoAction => {}
            WaitForVenue => {}
            Place { order_id, desired } => {
                actions.push(self.place_action(order_id.clone(), instrument, desired))
            }
            Cancel { order_id } => actions.push(self.cancel_action(order_id.clone(), instrument)),
            Replace {
                old_order_id,
                new_order_id,
                desired,
            } => {
                actions.push(self.cancel_action(old_order_id.clone(), instrument));
                actions.push(self.place_action(new_order_id.clone(), instrument, desired));
            }
        }

        actions
    }

    fn place_action(
        &self,
        order_id: String,
        instrument: &Instrument,
        desired: &Quote,
    ) -> OrderAction {
        OrderAction::Place(Order {
            order_id,
            instrument: instrument.clone(),
            side: self.side,
            price: desired.price,
            quantity: desired.quantity,
            order_type: OrderType::PostOnlyLimit,
        })
    }

    fn cancel_action(&self, order_id: String, instrument: &Instrument) -> OrderAction {
        OrderAction::Cancel {
            order_id,
            instrument: instrument.clone(),
            side: self.side,
        }
    }

    fn apply_optimistic(&mut self, decision: SidePlan, now: Instant) {
        match (self.state.clone(), decision) {
            (_, SidePlan::NoAction) => {}
            (_, SidePlan::WaitForVenue) => {}

            (OrderSideState::NoOrder, SidePlan::Place { order_id, desired }) => {
                self.state = OrderSideState::Placing {
                    order_id,
                    requested: desired,
                };
                self.last_update = Some(now);
            }

            (OrderSideState::Live { resting, .. }, SidePlan::Cancel { order_id }) => {
                self.state = OrderSideState::Cancelling { order_id, resting };
                self.last_update = Some(now);
            }

            // keep your existing “cancel + place immediately” behavior
            (
                _,
                SidePlan::Replace {
                    new_order_id,
                    desired,
                    ..
                },
            ) => {
                self.state = OrderSideState::Placing {
                    order_id: new_order_id,
                    requested: desired,
                };
                self.last_update = Some(now);
            }

            _ => {}
        }
    }
}

fn price_to_ticks(price: f64, tick: f64) -> i64 {
    if tick <= 0.0 {
        return 0;
    }
    (price / tick).round() as i64
}

fn generate_order_id(instrument: &Instrument, side: Side) -> String {
    Uuid::new_v4().to_string()
}
