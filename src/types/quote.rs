use crate::types::price::Price;

#[derive(Debug, Copy, Clone)]
pub struct Quote {
    pub price: Price,
    pub quantity: f64,
}
