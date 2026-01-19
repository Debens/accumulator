use crate::scheduling::{schedule_context::ScheduleContext, types::SkipReason};

pub trait SchedulePolicy {
    fn should_evaluate(&mut self, ctx: &ScheduleContext<'_>) -> Option<SkipReason>;
}
