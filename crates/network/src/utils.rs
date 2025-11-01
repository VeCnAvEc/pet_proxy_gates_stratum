use std::sync::Arc;
use std::sync::atomic::Ordering::Relaxed;
use serde_json::Value;
use tokio::sync::{oneshot, Mutex};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;

use tracing::{error, info, warn};
use score::job::ProxyMessage;
use crate::connection::{TOTAL_JOBS, TOTAL_JOBS_FAILED, TOTAL_JOBS_SUCCEEDED};

#[derive(Debug)]
pub enum Outcome {
    Replied,
    NoReply,
    Cancelled,
    IoError(tokio::io::Error)
}

pub async fn await_and_replay(
    writer: Arc<Mutex<BufWriter<OwnedWriteHalf>>>,
    rx: oneshot::Receiver<ProxyMessage<'static>>,
    cancel: CancellationToken
) -> Outcome {
    tokio::select! {
        _ = cancel.cancelled() => {
            Outcome::Cancelled
        }
        res = rx => {
            match res {
                Ok(msg) => {
                    match msg {
                        ProxyMessage::Wait => {
                            Outcome::NoReply
                        },
                        ProxyMessage::Response(response) => {
                            if let Err(err) = writer.lock().await.write_all(response.as_bytes()).await {
                                Outcome::IoError(err)
                            } else {
                                Outcome::Replied
                            }
                        },
                        ProxyMessage::Err(_) => {
                            Outcome::NoReply
                        }
                        _ => {
                            Outcome::NoReply
                        }
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