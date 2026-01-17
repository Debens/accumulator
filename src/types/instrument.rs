use std::fmt;

use anyhow::Result;

use crate::types::trading_rules::TradingRules;

#[derive(Clone)]
pub struct Instrument {
    base: String,
    quote: String,
    trading_rules: TradingRules,
}

impl Instrument {
    pub fn new(base: String, quote: String, trading_rules: TradingRules) -> Self {
        Self {
            base: base.to_uppercase(),
            quote: quote.to_uppercase(),
            trading_rules,
        }
    }

    pub fn from_str(symbol: &str) -> Result<Self> {
        if let Some((base, quote)) = symbol.split_once('/') {
            return Self::load(base.to_string(), quote.to_string());
        }

        anyhow::bail!("invalid instrument symbol: {symbol}");
    }

    pub fn load(base: String, quote: String) -> Result<Self> {
        let trading_rules = TradingRules::from_config(base.as_str(), quote.as_str())?;

        Ok(Self::new(base, quote, trading_rules))
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    pub fn quote(&self) -> &str {
        &self.quote
    }

    pub fn trading_rules(&self) -> &TradingRules {
        &self.trading_rules
    }
}

impl fmt::Display for Instrument {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}", self.base, self.quote)
    }
}

impl fmt::Debug for Instrument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Instrument({})", self)
    }
}
