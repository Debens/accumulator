use std::sync::Mutex;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::broadcast;

use crate::{
    execution::order_report::OrderReport,
    scheduling::{
        schedule_context::ScheduleContext, schedule_policy::SchedulePolicy, types::SkipReason,
    },
};

pub struct MinIntervalPolicy {
    min_interval: Duration,
    last_order: Arc<Mutex<Option<Instant>>>,
}

impl Clone for MinIntervalPolicy {
    fn clone(&self) -> Self {
        Self {
            min_interval: self.min_interval,
            last_order: Arc::clone(&self.last_order),
        }
    }
}

impl MinIntervalPolicy {
    pub fn new(min_interval: Duration) -> Self {
        Self {
            min_interval,
            last_order: Arc::new(Mutex::new(None)),
        }
    }

    pub fn on_report(&self, mut receiver: broadcast::Receiver<OrderReport>) {
        let last_eval = Arc::clone(&self.last_order);

        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Ok(report) => {
                        if let OrderReport::Placed { .. } = report {
                            *last_eval.lock().unwrap() = Some(Instant::now());
                        }
                    }
                }
            }
        });
    }
}

impl SchedulePolicy for MinIntervalPolicy {
    fn should_evaluate(&mut self, ctx: &ScheduleContext<'_>) -> Option<SkipReason> {
        let last = *self.last_order.lock().unwrap();

        if let Some(last) = last {
            let duration_since_last = ctx.now.duration_since(last);
            if duration_since_last < self.min_interval {
                return Some(SkipReason::TooSoon {
                    duration_since_last,
                });
            }
        }

        None
    }
}
