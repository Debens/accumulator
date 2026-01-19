use std::time::{Duration, Instant};

use crate::scheduling::{
    schedule_context::ScheduleContext, schedule_policy::SchedulePolicy, types::SkipReason,
};

pub struct MinIntervalPolicy {
    min_interval: Duration,
    last_eval: Option<Instant>,
}

impl MinIntervalPolicy {
    pub fn new(min_interval: Duration) -> Self {
        Self {
            min_interval,
            last_eval: None,
        }
    }
}

impl SchedulePolicy for MinIntervalPolicy {
    fn should_evaluate(&mut self, ctx: &ScheduleContext<'_>) -> Option<SkipReason> {
        if let Some(last) = self.last_eval {
            if ctx.now.duration_since(last) < self.min_interval {
                return Some(SkipReason::TooSoon);
            }
        }

        self.last_eval = Some(ctx.now);

        None
    }
}
