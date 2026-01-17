use std::fmt;

use crate::execution::order_action::OrderAction;
use crate::risk::context::RiskContext;
use crate::risk::decision::{RiskDecision, RiskHold, RiskReason, RiskRejection};
use crate::types::quote_target::QuoteTarget;

pub trait RiskCheck: Send + Sync {
    fn name(&self) -> &'static str;
    fn evaluate(&mut self, context: &RiskContext) -> Result<(), Vec<RiskReason>>;
}

pub struct RiskEngine {
    checks: Vec<Box<dyn RiskCheck>>,
}

impl RiskEngine {
    pub fn new(checks: Vec<Box<dyn RiskCheck>>) -> Self {
        Self { checks }
    }

    pub fn evaluate(
        &mut self,
        context: &RiskContext,
        proposed_target: QuoteTarget,
    ) -> RiskDecision {
        let mut reasons: Vec<RiskReason> = Vec::new();

        for check in &mut self.checks {
            if let Err(mut check_reasons) = check.evaluate(context) {
                reasons.append(&mut check_reasons);
            }
        }

        if reasons.is_empty() {
            return RiskDecision::Approved(proposed_target);
        }

        let is_hard_rule = reasons.iter().any(|reason| match reason {
            RiskReason::KillSwitchEnabled => true,
            RiskReason::MarketDataStale => true,
            RiskReason::CrossedOrInvalidBook => true,
            _ => false,
        });

        if is_hard_rule {
            return RiskDecision::Rejected(RiskRejection {
                reasons,
                required_actions: vec![OrderAction::CancelAll],
            });
        }

        RiskDecision::Hold(RiskHold { reasons })
    }
}

impl fmt::Debug for RiskEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RiskEngine")
            .field("checks_count", &self.checks.len())
            .finish()
    }
}
