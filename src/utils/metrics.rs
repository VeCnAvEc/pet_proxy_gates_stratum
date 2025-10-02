use std::sync::atomic::Ordering;
use std::sync::atomic::Ordering::Relaxed;
use tracing::error;
use crate::network::connection::{TOTAL_JOBS, TOTAL_JOBS_FAILED, TOTAL_JOBS_SUCCEEDED};
use crate::utils::socket::Outcome;

pub fn metrics_record_job_outcome(outcome: Outcome) {
    match outcome {
        Outcome::Replied => {
            TOTAL_JOBS_SUCCEEDED.fetch_add(1, Relaxed);
        }
        Outcome::IoError(err) => {
            error!("Outcome IoError: {:?}", err);
            TOTAL_JOBS_FAILED.fetch_add(1, Relaxed);
        }
        Outcome::NoReply | Outcome::Cancelled => {
            TOTAL_JOBS_FAILED.fetch_add(1, Relaxed);
        }
    }

    TOTAL_JOBS.fetch_add(1, Relaxed);
}