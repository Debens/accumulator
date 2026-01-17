use std::time::Instant;

#[derive(Debug, Clone)]
pub struct Ema {
    tau_seconds: f64,
    value: Option<f64>,
    last_update: Option<Instant>,
}

impl Ema {
    pub fn new(tau_seconds: f64) -> Self {
        Self {
            tau_seconds,
            value: None,
            last_update: None,
        }
    }

    pub fn update(&mut self, now: Instant, sample: f64) -> f64 {
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
}
