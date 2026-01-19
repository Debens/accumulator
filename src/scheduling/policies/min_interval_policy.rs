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
            let duration_since_last = ctx.now.duration_since(last);
            if duration_since_last < self.min_interval {
                return Some(SkipReason::TooSoon {
                    duration_since_last,
                });
            }
        }

        if ctx.order_manager.has_live_orders() || ctx.order_manager.has_inflight_actions() {
            self.last_eval = Some(ctx.now);
        }

        None
    }
}
