use crate::types::quote::Quote;

#[derive(Debug, Clone)]
pub enum SidePlan {
    NoAction,
    WaitForVenue,
    Place { order_id: String, desired: Quote },
    Cancel { order_id: String },
    Replace { order_id: String, desired: Quote },
}

#[derive(Debug, Clone)]
pub enum OrderSideState {
    NoOrder,
    Placing { order_id: String, requested: Quote },
    Live { order_id: String, resting: Quote },
    Cancelling { order_id: String, resting: Quote },
}

#[derive(Debug, Clone)]
pub struct OpenOrder {
    pub order_id: String,
}
