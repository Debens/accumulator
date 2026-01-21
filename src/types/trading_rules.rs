use crate::types::price::Price;
use crate::types::trading_hours::TradingHours;

use anyhow::{anyhow, bail, Context, Result};
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct TradingRules {
    /// Minimum price increment in quote currency (GBP).
    pub price_tick: f64,

    /// Minimum quantity increment in base currency (e.g. BTC).
    pub quantity_step: f64,

    /// Minimum half-spread in quote currency (GBP). Acts as a floor.
    pub min_half_spread: f64,

    /// Max notional per order in quote currency (GBP). Keeps risk stable as price moves.
    pub max_order_notional: f64,

    /// Max absolute exposure in quote currency (GBP).
    pub max_exposure_in_quote: f64,

    /// Optional trading hours restriction (UTC)
    #[serde(default)]
    pub trading_hours: Option<TradingHours>,
}

impl TradingRules {
    pub fn from_config(base: &str, quote: &str) -> Result<Self> {
        let key = format!("{base}_{quote}");
        Config::load()?
            .trading_rules
            .get(&key)
            .ok_or_else(|| anyhow!("unsupported trading pair, missing trading rules for \"{key}\""))
            .map(|rules| *rules)
    }

    pub fn round_price_to_tick(self, price: f64) -> Price {
        Price::new(round_down_to_step(price, self.price_tick))
    }

    pub fn round_quantity_to_step(self, quantity_base: f64) -> f64 {
        round_down_to_step(quantity_base, self.quantity_step)
    }

    pub fn quantity_from_notional(self, notional: f64, price_per_base: f64) -> f64 {
        if price_per_base <= 0.0 || !price_per_base.is_finite() {
            return 0.0;
        }
        let raw_quantity = notional / price_per_base;
        self.round_quantity_to_step(raw_quantity)
    }

    fn validate(&self) -> Result<()> {
        if self.price_tick <= 0.0 {
            bail!("price_tick must be > 0");
        }
        if self.quantity_step <= 0.0 {
            bail!("quantity_step must be > 0");
        }
        if self.min_half_spread < 0.0 {
            bail!("min_half_spread must be >= 0");
        }
        if self.max_order_notional <= 0.0 {
            bail!("max_order_notional must be > 0");
        }
        if self.max_exposure_in_quote <= 0.0 {
            bail!("max_exposure_in_quote must be > 0");
        }
        Ok(())
    }
}

fn round_down_to_step(value: f64, step: f64) -> f64 {
    if step <= 0.0 || !value.is_finite() || !step.is_finite() {
        return value;
    }

    (value / step).floor() * step
}

#[derive(Debug, Deserialize)]
struct Config {
    pub trading_rules: HashMap<String, TradingRules>,
}

static CONFIG: OnceCell<Config> = OnceCell::new();

impl Config {
    const FILE_NAME: &'static str = "trading_rules.yml";

    fn load() -> Result<&'static Config> {
        CONFIG.get_or_try_init(|| {
            let raw = fs::read_to_string(Self::FILE_NAME)
                .with_context(|| format!("failed to read trading rules {}", Self::FILE_NAME))?;

            let config: Config = serde_yaml::from_str::<Config>(&raw)
                .with_context(|| format!("failed to parse trading rules {}", Self::FILE_NAME))?;

            config
                .validate()
                .context("trading rules config validation failed")?;

            Ok(config)
        })
    }

    fn validate(&self) -> Result<()> {
        if self.trading_rules.is_empty() {
            bail!("trading_rules must not be empty");
        }
        for (pair, rules) in &self.trading_rules {
            rules
                .validate()
                .with_context(|| format!("invalid trading_rules for pair {pair}"))?;
        }
        Ok(())
    }
}
