use crate::scheduling::{
    schedule_context::ScheduleContext, schedule_policy::SchedulePolicy, types::SkipReason,
};

pub struct InFlightPolicy;

impl SchedulePolicy for InFlightPolicy {
    fn should_evaluate(&mut self, ctx: &ScheduleContext<'_>) -> Option<SkipReason> {
        if ctx.order_manager.has_inflight_actions() {
            Some(SkipReason::InFlight)
        } else {
            None
        }
    }
}
