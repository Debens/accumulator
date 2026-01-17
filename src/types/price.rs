use std::fmt;
use std::ops::{Add, Sub};

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct Price(f64);

impl Price {
    pub fn new(value: f64) -> Self {
        assert!(value.is_finite(), "price must be finite");
        assert!(value >= 0.0, "price must be non-negative");

        Price(value)
    }

    pub fn as_f64(self) -> f64 {
        self.0
    }
}

impl fmt::Display for Price {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:.2}", self.0)
    }
}

impl From<f64> for Price {
    fn from(value: f64) -> Self {
        Price::new(value)
    }
}

impl Add<f64> for Price {
    type Output = Price;

    fn add(self, rhs: f64) -> Price {
        Price::new(self.0 + rhs)
    }
}

impl Sub<f64> for Price {
    type Output = Price;

    fn sub(self, rhs: f64) -> Price {
        Price::new(self.0 - rhs)
    }
}

impl Sub for Price {
    type Output = f64;

    fn sub(self, rhs: Price) -> f64 {
        self.0 - rhs.0
    }
}
