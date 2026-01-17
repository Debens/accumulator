use crate::types::instrument::Instrument;

#[derive(Debug, Clone)]
pub struct InstrumentContext {
    pub instrument: Instrument,
}

impl InstrumentContext {
    pub fn new(instrument: &Instrument) -> Self {
        Self {
            instrument: instrument.clone(),
        }
    }

    pub fn tick(&self) -> f64 {
        self.rules().price_tick
    }

    pub fn max_order_notional(&self) -> f64 {
        self.rules().max_order_notional
    }

    pub fn min_half_spread(&self) -> f64 {
        self.rules().min_half_spread
    }

    pub fn rules(&self) -> &crate::types::trading_rules::TradingRules {
        self.instrument.trading_rules()
    }
}

pub trait WithContext {
    fn ctx(&self) -> &InstrumentContext;
}
