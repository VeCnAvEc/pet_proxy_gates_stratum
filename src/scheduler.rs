use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::{mpsc, oneshot, Semaphore};
use tokio::sync::mpsc::error::TryRecvError;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use crate::domain::job::{Job, JobRequest};
use crate::methods::submit::Submit;

const HIGH_BUDGET: u16 = 32;

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
        let mut remaining_high = HIGH_BUDGET;

        'outer: loop {
            if self.shutdown.is_cancelled() { break 'outer; }

            while remaining_high > 0 {
                info!(?remaining_high, "before high");
                match self.rx_high.try_recv() {
                    Ok(job) => {
                        self.process_high_queue(job).await;
                        remaining_high -= 1;

                        info!(?remaining_high, "after high");
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {}
                }

                if self.shutdown.is_cancelled() { break 'outer; }
            }

            let shutdown = self.shutdown.clone();
            if remaining_high > 0 {
                select! {
                    biased;
                    _ = shutdown.cancelled() => break 'outer,
                    high_job = self.rx_high.recv() => {
                        let job = high_job.unwrap();
                        info!(?remaining_high, "before high (select)");
                        self.process_high_queue(job).await;
                        remaining_high = remaining_high.saturating_sub(1);
                        info!(?remaining_high, "after high (select)");
                    }
                    norm_job = self.rx_norm.recv() => {
                        if let Some(job) = norm_job {
                            info!(
                                old_budget = remaining_high,
                                reset_to = HIGH_BUDGET,
                                "norm fired; budget reset"
                            );
                            self.process_norm_queue(job).await;
                            remaining_high = HIGH_BUDGET;
                        }
                    }
                }
            } else {
                select! {
                    _ = shutdown.cancelled() => break 'outer,
                    norm_job = self.rx_norm.recv() => {
                        if let Some(job) = norm_job {
                            info!(
                                old_budget = remaining_high,
                                reset_to = HIGH_BUDGET,
                                "norm fired; budget reset"
                            );
                            self.process_norm_queue(job).await;
                            remaining_high = HIGH_BUDGET;
                        }
                    }
                }
            }
            // select! {
            //     biased;
            //     _ = shutdown.cancelled() => {
            //         info!("break!!");
            //         break 'outer;
            //     }
            //
            //     high_job = self.rx_high.recv() => {
            //         let job = high_job.unwrap();
            //         info!(?remaining_high, "before high (select)");
            //         self.process_high_queue(job).await;
            //         remaining_high = remaining_high.saturating_sub(1);
            //         info!(?remaining_high, "after high (select)");
            //     }
            //
            //     norm_job = self.rx_norm.recv() => {
            //         if let Some(job) = norm_job {
            //             info!(
            //                 old_budget = remaining_high,
            //                 reset_to = HIGH_BUDGET,
            //                 "norm fired; budget reset"
            //             );
            //             self.process_norm_queue(job).await;
            //             remaining_high = HIGH_BUDGET;
            //         }
            //     }
            // }
        }

        Ok(())
    }

    pub async fn process_high_queue(&self, job: JobRequest) {
        match job.job {
            Job::MiningSubmit(submit) => {
                let _ = self.handle_submit(submit, job.respond_to).await;
            }
            _ => {
                warn!("It isn't a high priority job!");
            }
        }
    }

    pub async fn process_norm_queue(&self, job: JobRequest) {
        match job.job {
            Job::Ping => {
                job.respond_to.send("PONG\n").unwrap();
            }
            _ => {
                warn!("It isn't a norm priority job!");
            }
        }
    }

    pub async fn handle_submit(&self, submit: Submit, respond_to: oneshot::Sender<&'static str>) -> anyhow::Result<()>{
        let permit = self.cpu_limit.clone().acquire_owned().await?;

        let _join_submit = tokio::task::spawn_blocking(move ||  {
            let _permit = permit;

            std::thread::sleep(Duration::from_millis(100));

            info!("Submit -> {:?}", submit);
            respond_to.send("OK\n").unwrap();
        });

        Ok(())
    }
}