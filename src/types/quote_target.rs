use crate::types::quote::Quote;

#[derive(Debug, Clone)]
pub struct QuoteTarget {
    pub bid: Option<Quote>,
    pub ask: Option<Quote>,
}

impl QuoteTarget {
    pub fn none() -> Self {
        Self {
            bid: None,
            ask: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum NoQuoteReason {
    MissingTopOfBook,
    MissingFairPrice,
    MissingMid,
    MissingEma,
    MissingSlowEma,
    BelowEntryThreshold {
        deviation_ticks: f64,
        threshold_ticks: f64,
    },
    InvalidQuantity,
    WouldCrossPostOnly,
    BothSidesSuppressedByExposure,
    PullbackNotMet,
}
