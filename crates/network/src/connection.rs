use std::sync::Arc;
use std::sync::atomic::{AtomicU64};
use bytes::BytesMut;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::{mpsc, mpsc::Sender, Mutex};
use tokio::sync::oneshot;

use tokio_util::sync::CancellationToken;

use tracing::{debug, info, warn};

use score::job::{Job, JobRequest, ProxyMessage};
use score::miner::Miner;
use crate::message::{parse_message::parse_message, Command};
use crate::server::ConnId;
use crate::utils::metrics_record_job_outcome;
use crate::utils::await_and_replay;

pub static TOTAL_JOBS: AtomicU64 = AtomicU64::new(0);
pub static TOTAL_JOBS_SUCCEEDED: AtomicU64 = AtomicU64::new(0);
pub static TOTAL_JOBS_FAILED: AtomicU64 = AtomicU64::new(0);

pub async fn handle_connection(
    mut socket: TcpStream, token: CancellationToken,
    conn_id: ConnId, tx_queue_high: Sender<JobRequest>,
    tx_queue_norm: Sender<JobRequest>, api_url: String
) -> anyhow::Result<()> {
    let socket_addr = socket.peer_addr()?;

    let (reader, writer) = socket.into_split();
    let mut buf = BytesMut::with_capacity(1024);

    let mut reader = BufReader::new(reader);
    let mut writer = Arc::new(Mutex::new(BufWriter::new(writer)));

    let (miner_tx, miner_rx) = mpsc::channel(12);

    let miner = Arc::new(Mutex::new(Miner::new(socket_addr, miner_tx)));

    let token_pool_messages = token.clone();
    process_pool_messages(miner_rx, token_pool_messages, conn_id).await;

    loop {
        let miner = Arc::clone(&miner);
        let child_token = token.clone();
        let mut tmp = [0u8;256];

        select! {
            _ = child_token.cancelled() => {
                info!(conn_id, "conn cancelled");
                break;
            }
            n = reader.read(&mut tmp) => {
                let n = n?;

                if n == 0 {
                    token.cancel();
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);

                info!(conn_id, "read {} bytes, buf len = {}", n, buf.len());
                info!(conn_id, "buf = {:?}", std::str::from_utf8(&buf));

                while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                    let miner = Arc::clone(&miner);

                    let line = buf.split_to(pos + 1);
                    let line = std::str::from_utf8(&line)?.trim();

                    if line.is_empty() { continue; }

                    match parse_message(line)? {
                        Command::Ping => {
                            let (once_tx, mut once_rx) = oneshot::channel::<ProxyMessage>();
                            let job_request = JobRequest {
                                job: Job::Ping,
                                respond_to: once_tx,
                            };

                            let child_token_clone = child_token.clone();
                            let writer_clone = Arc::clone(&writer);
                            tokio::spawn(async move {
                                let outcome = await_and_replay(writer_clone, once_rx, child_token_clone).await;
                                metrics_record_job_outcome(outcome);
                            });

                            tx_queue_norm.send(job_request).await?;
                        },
                        Command::CSubmit(submit) => {
                            let (once_tx, mut once_rx) = oneshot::channel::<ProxyMessage>();
                            let job_request = JobRequest {
                                job: Job::MiningSubmit((submit, miner)),
                                respond_to: once_tx,
                            };

                            let child_token_clone = child_token.clone();
                            let writer_clone = Arc::clone(&writer);
                            tokio::spawn(async move {
                                let outcome = await_and_replay(writer_clone, once_rx, child_token_clone.clone()).await;
                                metrics_record_job_outcome(outcome);
                            });

                            tx_queue_high.send(job_request).await?;
                        },
                        Command::CSubscribe(subscribe) => {
                            let (once_tx, mut once_rx) = oneshot::channel::<ProxyMessage>();
                            let job_request = JobRequest {
                                job: Job::MiningSubscribe((subscribe, miner)),
                                respond_to: once_tx
                            };

                            let child_token_clone = child_token.clone();
                            let writer_clone = Arc::clone(&writer);
                            tokio::spawn(async move {
                                let outcome = await_and_replay(writer_clone, once_rx, child_token_clone).await;
                                metrics_record_job_outcome(outcome);
                            });

                            tx_queue_high.send(job_request).await?;
                        },
                        Command::CAuthorize(authorize) => {
                            let (once_tx, mut once_rx) = oneshot::channel::<ProxyMessage>();
                            let job_request = JobRequest {
                                job: Job::MiningAuthorize((authorize, miner)),
                                respond_to: once_tx
                            };

                            let child_token_clone = child_token.clone();
                            let writer_clone = Arc::clone(&writer);
                            tokio::spawn(async move {
                                let outcome = await_and_replay(writer_clone, once_rx, child_token_clone).await;
                                metrics_record_job_outcome(outcome);
                            });

                            tx_queue_high.send(job_request).await?;
                        }
                        Command::Unknown => {
                            info!("line: {:?}", line);
                            writer.lock().await.write_all(b"BAD COMMAND\n").await?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

async fn process_pool_messages(mut miner_rx: mpsc::Receiver<String>, token: CancellationToken, conn_id: ConnId) {
    let notify_handle =  tokio::spawn(async move {
        loop {
            select! {
                _ = token.cancelled() => {
                    info!("process miner notify is closed for connId: {}", conn_id);
                    break;
                }
                msg = miner_rx.recv() => {
                    match msg {
                        None => {
                            warn!("msg from pool is none");
                        }
                        Some(msg) => {
                            let json = serde_json::from_str::<Value>(msg.as_str());
                            info!("json from pool -> {:?}", json);
                        }
                    };

                }
            }
        }
    });
}