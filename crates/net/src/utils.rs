use std::sync::atomic::Ordering::Relaxed;
use futures::channel::oneshot;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;

use tracing::{error, info, warn};

use crate::connection::{TOTAL_JOBS, TOTAL_JOBS_FAILED, TOTAL_JOBS_SUCCEEDED};

#[derive(Debug)]
pub enum Outcome {
    Replied,
    NoReply,
    Cancelled,
    IoError(tokio::io::Error)
}

pub async fn await_and_replay(
    socket: &mut TcpStream,
    rx: &mut oneshot::Receiver<&'static str>,
    cancel: CancellationToken
) -> Outcome {
    tokio::select! {
        _ = cancel.cancelled() => {
            Outcome::Cancelled
        }
        res = rx => {
            match res {
                Ok(msg) => {
                    if let Err(err) = socket.write_all(msg.as_bytes()).await {
                        Outcome::IoError(err)
                    } else {
                        Outcome::Replied
                    }
                }
                Err(_err) => {Outcome::NoReply}
            }
        }
    }
}

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