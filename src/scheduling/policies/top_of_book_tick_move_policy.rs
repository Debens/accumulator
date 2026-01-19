use crate::scheduling::{
    schedule_context::ScheduleContext, schedule_policy::SchedulePolicy, types::SkipReason,
};

pub struct TopOfBookTickMovePolicy {
    min_ticks: f64,
    last_best: Option<(f64, f64)>,
}

impl TopOfBookTickMovePolicy {
    pub fn new(min_ticks: f64) -> Self {
        Self {
            min_ticks,
            last_best: None,
        }
    }
}

impl SchedulePolicy for TopOfBookTickMovePolicy {
    fn should_evaluate(&mut self, ctx: &ScheduleContext<'_>) -> Option<SkipReason> {
        let (best_bid, best_ask) =
            match ctx.market_state.best_bid().zip(ctx.market_state.best_ask()) {
                Some((b, a)) => (b.as_f64(), a.as_f64()),
                None => return Some(SkipReason::NoMeaningfulChange), // or a dedicated "NoBook"
            };

        let tick = ctx.instrument.trading_rules().price_tick;
        let min_move = self.min_ticks * tick;

        let moved = match self.last_best {
            Some((pb, pa)) => {
                (best_bid - pb).abs() >= min_move || (best_ask - pa).abs() >= min_move
            }
            None => true,
        };

        self.last_best = Some((best_bid, best_ask));

        if moved {
            None
        } else {
            Some(SkipReason::NoMeaningfulChange)
        }
    }
}
