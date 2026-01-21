use crate::execution::order_action::OrderAction;
use crate::types::quote_target::QuoteTarget;

#[derive(Debug, Clone)]
pub enum RiskDecision {
    Approved(QuoteTarget),
    Hold(RiskHold),
    Rejected(RiskRejection),
}

#[derive(Debug, Clone)]
pub struct RiskHold {
    pub reasons: Vec<RiskReason>,
}

#[derive(Debug, Clone)]
pub struct RiskRejection {
    pub reasons: Vec<RiskReason>,
    pub required_actions: Vec<OrderAction>,
}

#[derive(Debug, Clone)]
pub enum RiskReason {
    KillSwitchEnabled,
    MarketDataStale,
    MissingMarketData,
    CrossedOrInvalidBook,
    ChurnThrottleBid,
    ChurnThrottleAsk,
    InsufficientEdge { half_spread: f64, required: f64 },
}
