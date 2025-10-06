use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::jobs_telemetry::jobs_telemetry;

pub mod jobs_telemetry;

pub struct ActivityTelemetry {
    is_jobs_telemetry: bool,
    is_cpu_telemetry: bool,
    frequency: Duration,
    shutdown: CancellationToken
}

impl ActivityTelemetry {
    pub fn new(telemetry_token: CancellationToken, frequency: Duration) -> Self {
        ActivityTelemetry {
            is_jobs_telemetry: false,
            is_cpu_telemetry: false,
            frequency,
            shutdown: telemetry_token
        }
    }

    pub async fn run_telemetry(&self) -> anyhow::Result<()> {
        let mut interval = tokio::time::interval(self.frequency);

        loop {
            if self.shutdown.is_cancelled() { break; }
            interval.tick().await;

            if self.is_jobs_telemetry {
                self.log_jobs_telemetry();
            }
        }

        Ok(())
    }

    fn log_jobs_telemetry(&self) {
        jobs_telemetry();
    }

    pub fn set_jobs_telemetry(mut self, is_jobs_telemetry: bool) -> Self {
        self.is_jobs_telemetry = is_jobs_telemetry;
        self
    }
}