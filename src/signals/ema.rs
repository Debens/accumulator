use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Ema {
    tau_seconds: f64,
    value: Option<f64>,
    last_update: Option<Instant>,
    first_update: Option<Instant>,
    warmup_duration: Duration,
}

impl Ema {
    pub fn new(tau_seconds: f64) -> Self {
        let warmup_duration = Duration::from_secs_f64((tau_seconds * 1.0).max(0.0));
        Self {
            tau_seconds,
            value: None,
            last_update: None,
            first_update: None,
            warmup_duration,
        }
    }

    pub fn update(&mut self, now: Instant, sample: f64) -> f64 {
        if self.first_update.is_none() {
            self.first_update = Some(now);
        }

        match (self.value, self.last_update) {
            (None, _) => {
                self.value = Some(sample);
                self.last_update = Some(now);
                sample
            }
            (Some(previous), Some(previous_time)) => {
                let dt_seconds = now.duration_since(previous_time).as_secs_f64().max(0.0);
                let alpha = 1.0 - (-dt_seconds / self.tau_seconds).exp();

                let updated = previous + alpha * (sample - previous);
                self.value = Some(updated);
                self.last_update = Some(now);
                updated
            }
            (Some(_), None) => {
                self.value = Some(sample);
                self.last_update = Some(now);
                sample
            }
        }
    }

    pub fn warmed_value(&self) -> Option<f64> {
        let Some(first) = self.first_update else {
            return None;
        };
        let Some(last) = self.last_update else {
            return None;
        };

        if last.duration_since(first) < self.warmup_duration {
            return None;
        }

        self.value
    }
}
