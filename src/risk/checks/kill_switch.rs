use crate::risk::context::RiskContext;
use crate::risk::decision::RiskReason;
use crate::risk::engine::RiskCheck;

#[derive(Debug, Clone)]
pub struct KillSwitchCheck {
    pub enabled: bool,
}

impl KillSwitchCheck {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

impl RiskCheck for KillSwitchCheck {
    fn name(&self) -> &'static str {
        "KillSwitchCheck"
    }

    fn evaluate(&mut self, _context: &RiskContext) -> Result<(), Vec<RiskReason>> {
        if self.enabled {
            return Err(vec![RiskReason::KillSwitchEnabled]);
        }
        Ok(())
    }
}
