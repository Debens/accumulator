use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub enum ScheduleDecision {
    Evaluate,
    Skip(SkipReason),
}

#[derive(Debug, Clone, Copy)]
pub enum SkipReason {
    TooSoon { duration_since_last: Duration },
    NoMeaningfulChange,
    NoBook,
    InFlight,
}
