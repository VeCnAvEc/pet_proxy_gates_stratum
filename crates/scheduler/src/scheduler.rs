use std::cmp::min;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use tokio::select;
use tokio::sync::{oneshot, Mutex};
use tokio::sync::{mpsc, AcquireError, OwnedSemaphorePermit, Semaphore};
use tokio::sync::mpsc::error::TryRecvError;
use tokio_util::sync::CancellationToken;
use tokio::task::{JoinHandle, JoinSet};

use tracing::{error, info, warn};

use config::Config;

use score::job::{Authorize, Job, JobRequest, Submit, Subscribe};
use score::miner::Miner;

use network::api::client::ApiClient;

const HIGH_BUDGET: u16 = 32;

static IN_FLIGHT_CPU: AtomicU64 = AtomicU64::new(0);

struct InFlightCpuGuard;

impl InFlightCpuGuard {
    fn new() -> Self {
        IN_FLIGHT_CPU.fetch_add(1, Ordering::Relaxed);
        InFlightCpuGuard
    }
}

impl Drop for InFlightCpuGuard {
    fn drop(&mut self) {
        IN_FLIGHT_CPU.fetch_sub(1, Ordering::Relaxed);
    }
}


pub struct Scheduler {
    rx_high: mpsc::Receiver<JobRequest>,
    rx_norm: mpsc::Receiver<JobRequest>,
    shutdown: CancellationToken,
    cpu_limit: Arc<Semaphore>,
    config: Arc<Config>,
    api_client: Arc<ApiClient>
}

impl Scheduler {
    pub fn new(
        rx_high: mpsc::Receiver<JobRequest>,
        rx_norm: mpsc::Receiver<JobRequest>,
        shutdown: CancellationToken,
        cpu_limit: Arc<Semaphore>,
        config: Arc<Config>,
        api_client: Arc<ApiClient>
    ) -> Self {
        Self {
            rx_high,
            rx_norm,
            shutdown,
            cpu_limit,
            config,
            api_client
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        info!("Scheduler started!");
        let mut remaining_high = HIGH_BUDGET;

        // let mut tasks_controller = JoinSet::new();

        'outer: loop {
            if self.shutdown.is_cancelled() { break 'outer; }

            while remaining_high > 0 {
                match self.rx_high.try_recv() {
                    Ok(job) => {
                        self.process_high_queue(job).await;
                        remaining_high -= 1;
                    }
                    Err(TryRecvError::Empty) => {},
                    Err(TryRecvError::Disconnected) => {
                        // Here we will call token.cancel() and destroy all task
                        warn!("The high queue has been closed");
                        break;
                    }
                }

                if self.shutdown.is_cancelled() { break 'outer; }
            }

            let shutdown = self.shutdown.clone();
            if remaining_high > 0 {
                select! {
                    biased;
                    _ = shutdown.cancelled() => break 'outer,
                    high_job = self.rx_high.recv() => {
                        if let Some(job) = high_job {
                            self.process_high_queue(job).await;
                            remaining_high = remaining_high.saturating_sub(1);
                        }
                    }
                }
            } else {
                select! {
                    biased;
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
                    high_job = self.rx_high.recv() => {
                        if let Some(job) = high_job {
                            self.process_high_queue(job).await;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn process_high_queue(&self, job: JobRequest) {
        match job.job {
            Job::MiningSubmit((submit, miner)) => {
                let _ = self.handle_submit(submit, job.respond_to).await;
            },
            Job::MiningSubscribe((subscribe, miner)) => {
                let _ = self.handle_subscribe(subscribe, job.respond_to, miner).await;
            }
            Job::MiningAuthorize((authorize, miner)) => {
                let _ = self.handle_authorize(authorize, job.respond_to, miner).await;
            }
            _ => {
                warn!("It isn't a high priority job!");
            }
        }
    }

    pub async fn process_norm_queue(&self, job: JobRequest) {
        match job.job {
            Job::Ping => {
                if let Err(err) = job.respond_to.send("PONG\n") {
                    warn!("Couldn't to send to respond_to!");
                    error!("respond_to send error: {}", err);
                };
            }
            _ => {
                warn!("It isn't a norm priority job!");
            }
        }
    }

    pub async fn handle_submit(&self, submit: Submit, respond_to: oneshot::Sender<&'static str>) -> anyhow::Result<()> {
        let permit = self.cpu_limit.clone().acquire_owned().await?;

        let _join_submit = tokio::task::spawn_blocking(move ||  {
            let _permit = permit;
            let _guard = InFlightCpuGuard::new();

            std::thread::sleep(Duration::from_millis(100));

            info!("Submit -> {:?}", submit);
            if let Err(err) = respond_to.send("OK\n") {
                warn!("Couldn't to send respond_to!");
                error!("respond_to send error: {}", err);
            }
        });

        Ok(())
    }

    pub async fn handle_subscribe(&self, subscribe: Subscribe, respond_to: oneshot::Sender<&'static str>, miner: Arc<Mutex<Miner>>) -> anyhow::Result<()> {
        miner.lock().await.set_is_subscribe(true);
        info!("Miner is subscribe!");

        info!("Subscribe message: {:?}", subscribe);

        // respond_to();
        Ok(())
    }

    pub async fn handle_authorize(&self, authorize: Authorize, respond_to: oneshot::Sender<&'static str>, miner: Arc<Mutex<Miner>>) -> anyhow::Result<()> {
        let worker_full_name = authorize.username();

        let subaccount_info = self.api_client.get_subaccount_info(worker_full_name.to_string()).await?;

        {
            let mut miner_guard = miner.lock().await;

            let current_time = SystemTime::now();
            let since_epoch = current_time.duration_since(UNIX_EPOCH)?.as_secs();

            miner_guard.set_time_authorize(since_epoch);
            miner_guard.set_pool_addr(subaccount_info.pool_target);
            miner_guard.set_worker_name(subaccount_info.sub_account_name);
            miner_guard.set_is_subscribe(true);
        }


        Ok(())

    }
}