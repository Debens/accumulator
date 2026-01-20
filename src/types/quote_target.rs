use crate::types::quote::Quote;

#[derive(Debug, Clone)]
pub struct QuoteTarget {
    pub bid: Option<Quote>,
    pub ask: Option<Quote>,
}

#[derive(Debug, Clone)]
pub enum NoQuoteReason {
    MissingTopOfBook,
    MissingFairPrice,
    MissingMid,
    MissingEma,
    InsufficientInventory {
        asset: String,
        required: f64,
        available: f64,
    },
    BelowEntryThreshold {
        deviation_ticks: f64,
        threshold_ticks: f64,
    },
    TooLongExposure {
        exposure_quote: f64,
        max_exposure_quote: f64,
    },
    TooShortExposure {
        exposure_quote: f64,
        max_exposure_quote: f64,
    },
    InvalidQuantity,
    WouldCrossPostOnly,
    BothSidesSuppressedByExposure,
}
