use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use tokio_util::sync::CancellationToken;

use tracing::{warn, Instrument};

use network::server::Server;
use scheduler::scheduler::Scheduler;
use score::job::JobRequest;
use telemetry::ActivityTelemetry;
use config::Config;

pub async fn run_app(
    socket_addr: SocketAddr, semaphore: Arc<Semaphore>,
    tx_cpu_queue_high: Sender<JobRequest>, tx_cpu_queue_norm: Sender<JobRequest>,
    rx_cpu_queue_high: Receiver<JobRequest>, rx_cpu_queue_norm: Receiver<JobRequest>,
    token_shutdown: CancellationToken
) -> anyhow::Result<()> {
    let config = Arc::new(Config::new());
    let config_srv = config.clone();
    let config_sdr = config.clone();

    let server = Server::new(
        socket_addr,
        tx_cpu_queue_high,
        tx_cpu_queue_norm,
        token_shutdown.clone(),
        config_srv
    ).await?;
    let scheduler = Scheduler::new(
        rx_cpu_queue_high,
        rx_cpu_queue_norm,
        token_shutdown.clone(),
        semaphore,
        config_sdr
    );
    let mut telemetry = ActivityTelemetry::new(token_shutdown.clone(), Duration::from_secs(10))
        .set_jobs_telemetry(true);
    let mut set = JoinSet::new();

    set.spawn(async move { server.server_run().await }.instrument(tracing::info_span!("server")));
    set.spawn(async move { scheduler.run().await }.instrument(tracing::info_span!("scheduler")));
    set.spawn(async move { telemetry.run_telemetry().await }.instrument(tracing::info_span!("telemetry")));

    tokio::select! {
        _ = token_shutdown.cancelled() => {}
        _res = set.join_next() => {
            match set.join_next().await {
                Some(Ok(_)) => { }
                Some(Err(e)) => return Err(e.into()),
                None => { }
            }
        }
    }
    token_shutdown.cancel();

    while let Some(res) = set.join_next().await {
        if let Err(e) = res { warn!(?e, "task error") }
    }

    Ok(())
}