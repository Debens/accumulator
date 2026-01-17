use crate::execution::order_action::Side;
use crate::types::instrument::Instrument;
use crate::types::price::Price;

#[derive(Debug, Clone)]
pub enum OrderReport {
    Placed {
        order_id: String,
        instrument: Instrument,
        side: Side,
        price: Price,
        quantity: f64,
    },

    Accepted {
        order_id: String,
        instrument: Instrument,
        side: Side,
        price: Price,
        quantity: f64,
    },

    Rejected {
        order_id: String,
        instrument: Instrument,
        side: Side,
        reason: String,
    },

    PartiallyFilled {
        order_id: String,
        instrument: Instrument,
        side: Side,
        price: Price,
        quantity: f64,
        cum_quantity: f64,
    },

    Filled {
        order_id: String,
        instrument: Instrument,
        side: Side,
        price: Price,
        quantity: f64,
        cum_quantity: f64,
    },

    Cancel {
        order_id: String,
        instrument: Instrument,
        side: Side,
    },

    Cancelled {
        order_id: String,
        instrument: Instrument,
        side: Side,
    },

    CancelFailed {
        order_id: String,
        instrument: Instrument,
        side: Side,
        reason: String,
    },

    CancelledAll {
        count: i64,
    },

    VenueError {
        message: String,
    },
}
