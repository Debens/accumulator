#[derive(Debug, Clone, Copy)]
pub enum ScheduleDecision {
    Evaluate,
    Skip(SkipReason),
}

#[derive(Debug, Clone, Copy)]
pub enum SkipReason {
    TooSoon,
    NoMeaningfulChange,
    InFlight,
}
