use std::fmt;
use std::time::{Duration, Instant};

use crate::events::MarketEvent;
use crate::types::price::Price;

#[derive(Clone)]
pub struct MarketState {
    best_bid: Option<Price>,
    best_ask: Option<Price>,
    last_trade_price: Option<Price>,
    last_event_instant: Option<Instant>,
}

impl MarketState {
    pub fn new() -> Self {
        Self {
            best_bid: None,
            best_ask: None,
            last_trade_price: None,
            last_event_instant: None,
        }
    }

    pub fn on_market_event(&mut self, event: &MarketEvent) {
        self.last_event_instant = Some(Instant::now());

        match event {
            MarketEvent::TopOfBook {
                best_bid, best_ask, ..
            } => {
                self.best_bid = Some(*best_bid);
                self.best_ask = Some(*best_ask);
            }
            MarketEvent::Trade { price, .. } => {
                self.last_trade_price = Some(*price);
            }
        }
    }

    pub fn best_bid(&self) -> Option<Price> {
        self.best_bid
    }

    pub fn best_ask(&self) -> Option<Price> {
        self.best_ask
    }

    pub fn mid_price(&self) -> Option<Price> {
        let bid = self.best_bid?.as_f64();
        let ask = self.best_ask?.as_f64();
        Some(Price::new((bid + ask) / 2.0))
    }

    pub fn spread(&self) -> Option<f64> {
        let bid = self.best_bid?.as_f64();
        let ask = self.best_ask?.as_f64();
        Some(ask - bid)
    }

    pub fn last_trade_price(&self) -> Option<Price> {
        self.last_trade_price
    }

    pub fn is_stale(&self, max_age: Duration) -> bool {
        match self.last_event_instant {
            Some(last) => last.elapsed() > max_age,
            None => true,
        }
    }
}

impl fmt::Debug for MarketState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MarketState")
            .field("best_bid", &self.best_bid)
            .field("mid_price", &self.mid_price())
            .field("best_ask", &self.best_ask)
            .field("last_trade_price", &self.last_trade_price)
            .field("last_event_instant", &self.last_event_instant)
            .field("is_stale", &self.is_stale(Duration::from_secs(60)))
            .finish()
    }
}
