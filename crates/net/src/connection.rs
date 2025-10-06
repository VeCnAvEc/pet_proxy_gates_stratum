
use std::sync::atomic::{AtomicU64};
use bytes::BytesMut;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::{mpsc::Sender};
use futures::channel::oneshot;

use tokio_util::sync::CancellationToken;

use tracing::{info};

use core::job::{Job, JobRequest};
use crate::message::{parse_message, Command};
use crate::server::ConnId;
use crate::utils::metrics_record_job_outcome;
use crate::utils::await_and_replay;

pub static TOTAL_JOBS: AtomicU64 = AtomicU64::new(0);
pub static TOTAL_JOBS_SUCCEEDED: AtomicU64 = AtomicU64::new(0);
pub static TOTAL_JOBS_FAILED: AtomicU64 = AtomicU64::new(0);

pub async fn handle_connection(
    mut socket: TcpStream, token: CancellationToken,
    conn_id: ConnId, tx_queue_high: Sender<JobRequest>,
    tx_queue_norm: Sender<JobRequest>
) -> anyhow::Result<()> {
    let mut buf = BytesMut::with_capacity(1024);
    loop {
        let child_token = token.clone();
        let mut tmp = [0u8;256];

        select! {
            _ = child_token.cancelled() => {
                info!(conn_id, "conn cancelled");
                break;
            }
            n = socket.read(&mut tmp) => {
                let n = n?;

                if n == 0 { break; }
                buf.extend_from_slice(&tmp[..n]);
                while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                    let line = buf.split_to(pos + 1);
                    let line = std::str::from_utf8(&line)?.trim();

                    if line.is_empty() { continue; }

                    match parse_message(line)? {
                        Command::Ping => {
                            let (once_tx, mut once_rx) = oneshot::channel::<&str>();
                            let job_request = JobRequest {
                                job: Job::Ping,
                                respond_to: once_tx,
                            };

                            tx_queue_norm.send(job_request).await?;

                            let outcome = await_and_replay(&mut socket, &mut once_rx, child_token.clone()).await;
                            metrics_record_job_outcome(outcome)
                        },
                        Command::CSubmit(submit) => {
                            let (once_tx, mut once_rx) = oneshot::channel::<&str>();
                            let job_request = JobRequest {
                                job: Job::MiningSubmit(submit),
                                respond_to: once_tx,
                            };

                            tx_queue_high.send(job_request).await?;

                            let outcome = await_and_replay(&mut socket, &mut once_rx, child_token.clone()).await;
                            metrics_record_job_outcome(outcome)
                        },
                        Command::Unknown => {
                            socket.write_all(b"BAD COMMAND\n").await?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}