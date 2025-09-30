use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::{mpsc, Semaphore};
use tokio_util::sync::CancellationToken;
use tracing::info;
use crate::domain::job::{Job, JobRequest};

pub struct Scheduler {
    rx_high: mpsc::Receiver<JobRequest>,
    rx_norm: mpsc::Receiver<JobRequest>,
    shutdown: CancellationToken,
    cpu_limit: Arc<Semaphore>
}

impl Scheduler {
    pub fn new(
        rx_high: mpsc::Receiver<JobRequest>,
        rx_norm: mpsc::Receiver<JobRequest>,
        shutdown: CancellationToken,
        cpu_limit: Arc<Semaphore>
    ) -> Self {
        Self {
            rx_high,
            rx_norm,
            shutdown,
            cpu_limit,
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        info!("Scheduler started!");
        loop {
            let shutdown = self.shutdown.clone();
            select! {
                _ = shutdown.cancelled() => {
                    info!("break!!");
                    break;
                }

                high_job = self.rx_high.recv() => {
                    info!("high_job -> {:?}", high_job);
                    let job = high_job.unwrap();
                    match job.job {
                        Job::MiningSubmit(submit) => {
                            job.respond_to.send("OK").unwrap();
                        }
                        Job::Ping => {
                            job.respond_to.send("PONG").unwrap();
                        }
                    }
                }

                norm_job = self.rx_norm.recv() => {
                    info!("norm_job -> {:?}", norm_job);
                    let job = norm_job.unwrap();
                    match job.job {
                        Job::MiningSubmit(submit) => {
                            job.respond_to.send("OK\n").unwrap();
                        }
                        Job::Ping => {
                            job.respond_to.send("PONG\n").unwrap();
                        }
                    }
                }
            }
        }

        Ok(())
    }
}