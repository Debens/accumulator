use std::fmt;
use std::str::FromStr;

use anyhow::{Result, anyhow};
use clap::ValueEnum;

use crate::{
    execution::{ExecutionVenue, ReportSender, dry_run::DryRunExecutionVenue},
    kraken::{kraken_config::KrakenConfig, kraken_venue::KrakenExecutionVenue},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum VenueKind {
    #[clap(name = "dry-run")]
    DryRun,
    Kraken,
}

impl fmt::Display for VenueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DryRun => write!(f, "dry-run"),
            Self::Kraken => write!(f, "kraken"),
        }
    }
}

impl FromStr for VenueKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "dry-run" | "dryrun" | "paper" => Ok(Self::DryRun),
            "kraken" => Ok(Self::Kraken),
            other => Err(anyhow!("unknown venue kind: {other}")),
        }
    }
}
