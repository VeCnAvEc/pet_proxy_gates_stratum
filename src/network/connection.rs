use std::time::Duration;
use bytes::BytesMut;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::{mpsc::Sender, oneshot};

use tokio_util::sync::CancellationToken;

use tracing::{error, info};

use crate::domain::job::{Job, JobRequest};
use crate::network::message::{parse_message, Command};
use crate::network::server::ConnId;

pub async fn handle_connection(
    mut socket: TcpStream, token: CancellationToken,
    conn_id: ConnId, tx_queue_high: Sender<JobRequest>,
    tx_norm: Sender<JobRequest>
) -> anyhow::Result<()> {
    let mut buf = BytesMut::with_capacity(1024);
    loop {
        let child_token = token.clone();
        let mut tmp = [0u8;256];

        select! {
            _ = child_token.cancelled() => {
                info!(conn_id, "conn cancelled");
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

                            tx_queue_high.send(job_request).await?;

                            loop {
                                let mut interval = tokio::time::interval(Duration::from_millis(1000));

                                match once_rx.try_recv() {
                                    Ok(result) => {
                                        info!("result -> {}", result);
                                        socket.write_all(result.as_bytes()).await?;
                                        break;
                                    }
                                    Err(err) => {
                                        error!(error=?err, "Channel");
                                    }
                                }

                                interval.tick().await;
                            }
                        },
                        Command::CSubmit(submit) => {
                            info!("It's a submit");
                            let (once_tx, mut once_rx) = oneshot::channel::<&str>();
                            let job_request = JobRequest {
                                job: Job::MiningSubmit(submit),
                                respond_to: once_tx,
                            };

                            tx_queue_high.send(job_request).await?;

                            loop {
                                let mut interval = tokio::time::interval(Duration::from_millis(1000));

                                match once_rx.try_recv() {
                                    Ok(result) => {
                                        info!("result -> {}", result);
                                        socket.write_all(result.as_bytes()).await?;
                                        break;
                                    }
                                    Err(err) => {error!(error=?err, "Channel")},
                                }
                                interval.tick().await;
                            }
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