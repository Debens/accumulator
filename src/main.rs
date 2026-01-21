mod events;
mod execution;
mod inventory;
mod kraken;
mod market;
mod risk;
mod scenario;
mod scheduling;
mod signals;
mod strategy;
mod types;

use std::time::{Duration, Instant};

use anyhow::Result;
use clap::Parser;
use dotenvy::dotenv;
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use crate::events::MarketEvent;
use crate::execution::order_action::OrderAction;
use crate::execution::order_manager::OrderManager;
use crate::execution::order_report::OrderReport;
use crate::kraken::kraken_market::KrakenMarket;
use crate::market::market_source::MarketDataSource;
use crate::market::market_state::MarketState;
use crate::risk::checks::min_edge::MinEdgeCheck;
use crate::risk::checks::{
    churn_throttle::ChurnThrottleCheck, kill_switch::KillSwitchCheck,
    market_freshness::MarketFreshnessCheck, market_sanity::MarketSanityCheck,
};
use crate::risk::context::RiskContext;
use crate::risk::decision::RiskDecision;
use crate::risk::engine::RiskEngine;
use crate::scenario::scenario::Scenario;
use crate::scenario::strategies::StrategyKind;
use crate::scenario::venues::VenueKind;
use crate::scheduling::policies::in_flight_policy::InFlightPolicy;
use crate::scheduling::policies::min_interval_policy::{self, MinIntervalPolicy};
use crate::scheduling::policies::top_of_book_tick_move_policy::TopOfBookTickMovePolicy;
use crate::scheduling::policies::trading_hours_policy::TradingHoursPolicy;
use crate::scheduling::quote_scheduler::QuoteScheduler;
use crate::scheduling::schedule_context::ScheduleContext;
use crate::scheduling::types::ScheduleDecision;
use crate::types::instrument::Instrument;

const STARTUP_ACTIONS: &[OrderAction] = &[OrderAction::CancelAll];

#[derive(Debug, Clone, Parser)]
struct Args {
    #[arg(long, value_enum, default_value = "dry-run")]
    pub venue: VenueKind,

    #[arg(long, value_enum, default_value = "mean-reversion")]
    pub strategy: StrategyKind,

    #[arg(long, default_value = "SOL")]
    pub base: String,

    #[arg(long, default_value = "GBP")]
    pub quote: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("accumulator=debug".parse().unwrap())
                .add_directive("market=debug".parse().unwrap())
                .add_directive("execution=info".parse().unwrap()),
        )
        .with_target(false)
        .with_thread_ids(true)
        .init();

    let args = Args::parse();
    let instrument = Instrument::load(args.base, args.quote)?;

    let (market_event_sender, mut market_event_receiver) = mpsc::channel::<MarketEvent>(10_000);
    let (order_report_sender, _) = broadcast::channel::<OrderReport>(10_000);
    let mut order_report_receiver = order_report_sender.subscribe();
    let mut order_report_log_receiver = order_report_sender.subscribe();

    tokio::spawn(async move {
        loop {
            match order_report_log_receiver.recv().await {
                Ok(report) => info!(?report),
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(lagged = n, "order report logger lagged; dropped messages");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    tokio::spawn({
        let instrument = instrument.clone();
        let kraken_source = KrakenMarket::default();
        async move {
            loop {
                if let Err(error) = kraken_source
                    .subscribe(&instrument, market_event_sender.clone())
                    .await
                {
                    error!("KrakenMarket stopped with error: {error:?}");
                }

                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    });

    let mut order_manager = OrderManager::default();

    let venue = Scenario::execution_venue(args.venue, order_report_sender.clone()).await?;
    venue.spawn_reports(order_report_sender.clone()).await?;

    let inventory_source = venue.spawn_inventory(&instrument).await?.subscribe();

    venue.execute(STARTUP_ACTIONS).await?;

    let strategy = Scenario::strategy(args.strategy, &instrument);
    let mut risk_engine = RiskEngine::new(vec![
        Box::new(KillSwitchCheck::new(false)),
        Box::new(MarketFreshnessCheck::new(Duration::from_secs(3))),
        Box::new(MarketSanityCheck::new()),
        Box::new(ChurnThrottleCheck::new(Duration::from_millis(800))),
        Box::new(MinEdgeCheck::for_instrument(&instrument)),
    ]);

    let mut market_state = MarketState::new();
    let mut signal_state = Scenario::signals(args.strategy);

    let min_interval_policy = MinIntervalPolicy::new(Duration::from_millis(200));
    min_interval_policy.on_report(order_report_sender.subscribe());

    let mut quote_scheduler = QuoteScheduler::new(vec![
        Box::new(InFlightPolicy),
        Box::new(TopOfBookTickMovePolicy::new(1.0)),
        Box::new(TradingHoursPolicy::for_instrument(&instrument)),
        Box::new(min_interval_policy),
    ]);

    loop {
        tokio::select! {
            report = order_report_receiver.recv() => {
                match report {
                    Ok(report) => order_manager.on_report(report),
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(lagged = n, "engine lagged on order reports; state may be stale until next report");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        error!("order report channel closed");
                        break;
                    }
                }
            }

            Some(event) = market_event_receiver.recv() => {
                tracing::debug!(?event);
                let now = Instant::now();

                market_state.on_market_event(&event);
                signal_state.update(&market_state, now);

                let scheduler_context = ScheduleContext {
                    now,
                    instrument: &instrument,
                    market_state: &market_state,
                    order_manager: &order_manager,
                };

                match quote_scheduler.decide(&scheduler_context) {
                    ScheduleDecision::Evaluate => {}
                    ScheduleDecision::Skip(reason) => {
                        warn!(?reason, "scheduling skipped");

                        continue;
                    }
                }

                let inventory = *inventory_source.borrow();

                let target_result = strategy.compute_target(&market_state, &signal_state, inventory);
                match target_result {
                    Err(reason) => warn!(?reason),
                    Ok(target) => {
                        let context = RiskContext {
                            instrument: &instrument,
                            market_state: &market_state,
                            target: &target,
                            now,
                        };

                        match risk_engine.evaluate(&context, target.clone()) {
                            RiskDecision::Approved(approved_target) => {
                                let actions = order_manager
                                    .actions_for_target(&instrument, &approved_target, now)
                                    .await?;

                                if !actions.is_empty() {
                                    venue.execute(&actions).await?;
                                }
                            }
                            RiskDecision::Hold(hold) => {
                                info!(reasons = ?hold.reasons, "throttling actions");
                            }
                            RiskDecision::Rejected(rejection) => {
                                warn!(
                                    reasons = ?rejection.reasons,
                                    required_actions = ?rejection.required_actions,
                                    "risk rejected quote target"
                                );

                                venue.execute(&rejection.required_actions).await?;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
