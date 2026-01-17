use anyhow::{Result, anyhow};

use crate::types::{instrument::Instrument, price::Price};
use std::{fmt, str::FromStr};

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub enum Side {
    #[default]
    Buy,
    Sell,
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Side::Buy => write!(f, "BUY"),
            Side::Sell => write!(f, "SELL"),
        }
    }
}

impl FromStr for Side {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "BUY" => Ok(Self::Buy),
            "SELL" => Ok(Self::Sell),
            other => Err(anyhow!("unknown side: {other}")),
        }
    }
}

#[derive(Debug, Clone)]
pub enum OrderType {
    PostOnlyLimit,
}

#[derive(Debug, Clone)]
pub struct Order {
    pub order_id: String,
    pub instrument: Instrument,
    pub side: Side,
    pub price: Price,
    pub quantity: f64,
    pub order_type: OrderType,
}

#[derive(Debug, Clone)]
pub enum OrderAction {
    CancelAll,
    Cancel {
        order_id: String,
        instrument: Instrument,
        side: Side,
    },
    Place(Order),
}
