use crate::scheduling::{
    schedule_context::ScheduleContext, schedule_policy::SchedulePolicy, types::ScheduleDecision,
};

pub struct QuoteScheduler {
    policies: Vec<Box<dyn SchedulePolicy + Send>>,
}

impl QuoteScheduler {
    pub fn new(policies: Vec<Box<dyn SchedulePolicy + Send>>) -> Self {
        Self { policies }
    }

    pub fn decide(&mut self, context: &ScheduleContext<'_>) -> ScheduleDecision {
        for policy in self.policies.iter_mut() {
            if let Some(reason) = policy.should_evaluate(&context) {
                return ScheduleDecision::Skip(reason);
            }
        }

        ScheduleDecision::Evaluate
    }
}
