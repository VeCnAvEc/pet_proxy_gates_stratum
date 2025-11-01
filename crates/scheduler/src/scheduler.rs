use std::borrow::Cow;
use std::cmp::min;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use anyhow::anyhow;
use serde_json::Value;

use tokio::select;
use tokio::sync::{oneshot, Mutex};
use tokio::sync::{mpsc, AcquireError, OwnedSemaphorePermit, Semaphore};
use tokio::sync::mpsc::error::TryRecvError;
use tokio_util::sync::CancellationToken;
use tokio::task::{JoinHandle, JoinSet};

use tracing::{error, info, warn};

use config::Config;

use score::job::{AuthorizeParams, Job, JobRequest, ProxyMessage, SubmitParams, SubscribeParams};
use score::miner::Miner;

use network::api::client::{ApiClient, ApiResponse};
use network::upstream::pool_client::PoolClient;

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

#[derive(Debug)]
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
                if let Err(err) = job.respond_to.send(ProxyMessage::Response(Cow::from("OK\n"))) {
                    warn!("Couldn't to send to respond_to!");
                    error!("respond_to send error: {:?}", err);
                };
            }
            _ => {
                warn!("It isn't a norm priority job!");
            }
        }
    }

    pub async fn handle_submit(&self, submit: SubmitParams, respond_to: oneshot::Sender<ProxyMessage<'static>>) -> anyhow::Result<()> {
        let permit = self.cpu_limit.clone().acquire_owned().await?;

        let _join_submit = tokio::task::spawn_blocking(move ||  {
            let _permit = permit;
            let _guard = InFlightCpuGuard::new();

            std::thread::sleep(Duration::from_millis(100));

            info!("Submit -> {:?}", submit);
            if let Err(err) = respond_to.send(ProxyMessage::Response(Cow::from("OK\n"))) {
                warn!("Couldn't to send respond_to!");
                error!("respond_to send error: {:?}", err);
            }
        });

        Ok(())
    }

    pub async fn handle_subscribe(&self, subscribe: SubscribeParams, respond_to: oneshot::Sender<ProxyMessage<'static>>, miner: Arc<Mutex<Miner>>) -> anyhow::Result<()> {
        let mut miner_guard = miner.lock().await;

        let subscribe_json = serde_json::to_string(&subscribe)?;

        if !miner_guard.is_authorize() {
            miner_guard.set_pending_subscribe(subscribe_json);
            info!("miner_guard pending subscribe: {:?}", miner_guard.take_pending_subscribe());
            info!("Miner not authorized yet. Subscribe saved for later.");
            let result = respond_to.send(ProxyMessage::Wait);
            if let Err(_) = result {
                return Err(anyhow!("Channel has been closed"));
            }

            return Ok(());
        }

        if let Some(pool_tx) = miner_guard.pool_tx() {
            pool_tx.send(subscribe_json).await?;
            miner.lock().await.set_is_subscribe(true);
        } else {
            warn!("Pool tx not available even though miner is authorized");
        }

        Ok(())
    }

    pub async fn handle_authorize(&self, authorize: AuthorizeParams, respond_to: oneshot::Sender<ProxyMessage<'static>>, miner: Arc<Mutex<Miner>>) -> anyhow::Result<()> {
        let worker_full_name = authorize.username();

        let subaccount_info = self.api_client.get_subaccount_info(worker_full_name.to_string()).await?;

        match subaccount_info {
            ApiResponse::Successfully(subaccount_info) => {
                let pool_target = subaccount_info.pool_target.clone();

                let authorize_json = serde_json::to_string(&authorize)?;
                let miner_tx = miner.lock().await.miner_tx();
                let pool_client = PoolClient::new(&pool_target, miner_tx).await?;

                {
                    let mut miner_guard = miner.lock().await;
                    let current_time = SystemTime::now();
                    let since_epoch = current_time.duration_since(UNIX_EPOCH)?.as_secs();

                    miner_guard.set_time_authorize(since_epoch);
                    miner_guard.set_pool_addr(subaccount_info.pool_target);
                    miner_guard.set_worker_name(subaccount_info.sub_account_name);
                    miner_guard.set_is_authorize(true);
                    miner_guard.set_pool_tx(pool_client.miner_channel_writer());

                    if let Some(pending_subscribe) = miner_guard.take_pending_subscribe() {
                        if let Some(pool_tx) = miner_guard.pool_tx() {
                            pool_tx.send(pending_subscribe.clone()).await?;
                        }
                    }
                }

                let sender = pool_client.miner_channel_writer();

                if let Err(e) = sender.send(authorize_json).await {
                    error!("Channel was closed with error: {:?}", e);
                }
            }
            ApiResponse::NotFoundSubAccount(error) => {
                info!("error -> {:?}", error);
            }
        }

        Ok(())
    }
}