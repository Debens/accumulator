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
    #[clap(name = "trend-following")]
    TrendFollowing,
    #[clap(name = "regime-switch")]
    RegimeSwitch,
}

impl fmt::Display for StrategyKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SimpleMarketMaker => write!(f, "simple-mm"),
            Self::MeanReversion => write!(f, "mean-reversion"),
            Self::TrendFollowing => write!(f, "trend-following"),
            Self::RegimeSwitch => write!(f, "regime-switch"),
        }
    }
}

impl FromStr for StrategyKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "simple-mm" => Ok(Self::SimpleMarketMaker),
            "mean-reversion" => Ok(Self::MeanReversion),
            "trend-following" => Ok(Self::TrendFollowing),
            "regime-switch" => Ok(Self::RegimeSwitch),
            other => Err(anyhow!("unknown strategy kind: {other}")),
        }
    }
}
