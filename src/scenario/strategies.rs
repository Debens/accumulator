use std::fmt;
use std::str::FromStr;

use anyhow::{Result, anyhow};
use clap::ValueEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum StrategyKind {
    #[clap(name = "simple-mm")]
    SimpleMarketMaker,
    #[clap(name = "mean-reversion")]
    MeanReversion,
}

impl fmt::Display for StrategyKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SimpleMarketMaker => write!(f, "simple-mm"),
            Self::MeanReversion => write!(f, "mean-reversion"),
        }
    }
}

impl FromStr for StrategyKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "simple-mm" => Ok(Self::SimpleMarketMaker),
            "mean-reversion" => Ok(Self::MeanReversion),
            other => Err(anyhow!("unknown strategy kind: {other}")),
        }
    }
}
