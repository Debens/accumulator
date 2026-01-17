use crate::types::price::Price;

#[derive(Debug, Clone, Copy, Default)]
pub struct Inventory {
    /// Base asset position (e.g., BTC). Positive = long BTC, negative = short BTC.
    pub base: f64,
    /// Quote cash (e.g., GBP). Positive = you hold GBP.
    pub quote: f64,
}

impl Inventory {
    pub fn new(base: f64, quote: f64) -> Self {
        Self { base, quote }
    }

    /// Mark-to-market value in quote currency using mid price.
    pub fn mtm_quote(&self, mid: Price) -> f64 {
        self.quote + self.base * mid.as_f64()
    }

    /// Base exposure in quote currency at mid.
    pub fn exposure_quote(&self, mid: Price) -> f64 {
        self.base * mid.as_f64()
    }
}
