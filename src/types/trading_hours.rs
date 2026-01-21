use serde::Deserialize;

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct TradingHours {
    /// Start hour in UTC (inclusive), 0–23
    pub start_hour: u8,

    /// End hour in UTC (exclusive), 1–24
    pub end_hour: u8,

    /// Whether to pause trading on Saturday/Sunday
    pub weekend_pause: bool,
}

impl Default for TradingHours {
    fn default() -> Self {
        Self {
            start_hour: 8, // 08:00 UTC
            end_hour: 20,  // 20:00 UTC
            weekend_pause: false,
        }
    }
}
