use std::sync::atomic::Ordering;

use log::info;

use net::connection::{TOTAL_JOBS, TOTAL_JOBS_FAILED, TOTAL_JOBS_SUCCEEDED};

pub fn jobs_telemetry() {
    info!("[JOBS] Total Jobs: {}", TOTAL_JOBS.load(Ordering::Relaxed));
    info!("[JOBS] Total succeeded jobs: {}", TOTAL_JOBS_SUCCEEDED.load(Ordering::Relaxed));
    info!("[JOBS] Total failed jobs: {}", TOTAL_JOBS_FAILED.load(Ordering::Relaxed));
}