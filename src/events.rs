use crate::types::{instrument::Instrument, price::Price};

#[derive(Debug, Clone)]
pub enum MarketEvent {
    Trade {
        instrument: Instrument,
        price: Price,
        quantity: f64,
        timestamp_ms: u64,
    },
    TopOfBook {
        instrument: Instrument,
        best_bid: Price,
        best_ask: Price,
        timestamp_ms: u64,
    },
}
