use crate::scheduling::{
    schedule_context::ScheduleContext, schedule_policy::SchedulePolicy, types::SkipReason,
};
use crate::types::instrument::Instrument;
use crate::types::trading_hours::TradingHours;
use chrono::Datelike;
use chrono::{Timelike, Utc, Weekday};

pub struct TradingHoursPolicy {
    pub trading_hours: TradingHours,
}

impl TradingHoursPolicy {
    pub fn new(trading_hours: TradingHours) -> Self {
        Self { trading_hours }
    }

    pub fn for_instrument(instrument: &Instrument) -> Self {
        let trading_hours = instrument.trading_rules().trading_hours.clone();

        Self::new(trading_hours.unwrap_or_default())
    }

    fn is_within_hours(&self, hour: u32) -> bool {
        let start = self.trading_hours.start_hour as u32;
        let end = self.trading_hours.end_hour as u32;
        if start <= end {
            (hour >= start) && (hour < end)
        } else {
            (hour >= start) || (hour < end)
        }
    }
}

impl SchedulePolicy for TradingHoursPolicy {
    fn should_evaluate(&mut self, _ctx: &ScheduleContext<'_>) -> Option<SkipReason> {
        let now = Utc::now();
        let hour = now.hour();
        let weekday = now.weekday();

        if self.trading_hours.weekend_pause {
            match weekday {
                Weekday::Sat => return Some(SkipReason::WeekendPause),
                Weekday::Sun => return Some(SkipReason::WeekendPause),
                _ => {}
            }
        }

        if !self.is_within_hours(hour) {
            return Some(SkipReason::OutOfTradingHours {
                start_hour: self.trading_hours.start_hour,
                end_hour: self.trading_hours.end_hour,
            });
        }

        None
    }
}
