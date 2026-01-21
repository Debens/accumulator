use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub enum ScheduleDecision {
    Evaluate,
    Skip(SkipReason),
}

#[derive(Debug, Clone, Copy)]
pub enum SkipReason {
    TooSoon { duration_since_last: Duration },
    NoMeaningfulChange { best_bid: f64, best_ask: f64 },
    NoBook,
    InFlight,
    OutOfTradingHours { start_hour: u8, end_hour: u8 },
    WeekendPause,
}
