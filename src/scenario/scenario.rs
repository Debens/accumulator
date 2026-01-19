use anyhow::Result;

use crate::{
    execution::{ExecutionVenue, ReportSender, dry_run::DryRunExecutionVenue},
    kraken::{kraken_config::KrakenConfig, kraken_venue::KrakenExecutionVenue},
    scenario::{strategies::StrategyKind, venues::VenueKind},
    signals::signal_state::SignalState,
    strategy::{
        strategies::{
            mean_reversion::MakerOnlyMeanReversionStrategy, simple_mm::SimpleMarketMakerStrategy,
        },
        strategy::Strategy,
    },
    types::instrument::Instrument,
};

pub struct Scenario;

type DynamicVenue = Box<dyn ExecutionVenue + Send + Sync>;

impl Scenario {
    pub async fn execution_venue(kind: VenueKind, on_report: ReportSender) -> Result<DynamicVenue> {
        tracing::info!(venue = %kind, "creating execution venue");

        let venue: Box<dyn ExecutionVenue + Send + Sync> = match kind {
            VenueKind::DryRun => Box::new(DryRunExecutionVenue::new(on_report)),
            VenueKind::Kraken => {
                let config = KrakenConfig::from_env()?;

                Box::new(KrakenExecutionVenue::new(config, on_report))
            }
        };

        Ok(venue)
    }

    pub fn strategy(kind: StrategyKind, instrument: &Instrument) -> Box<dyn Strategy> {
        tracing::info!(strategy = %kind, "creating strategy");

        match kind {
            StrategyKind::SimpleMarketMaker => {
                Box::new(SimpleMarketMakerStrategy::for_instrument(instrument))
            }
            StrategyKind::MeanReversion => {
                Box::new(MakerOnlyMeanReversionStrategy::for_instrument(instrument))
            }
        }
    }

    pub fn signals(kind: StrategyKind) -> SignalState {
        match kind {
            StrategyKind::SimpleMarketMaker => SignalState::new(3.0),
            StrategyKind::MeanReversion => SignalState::new(60.0),
        }
    }
}
