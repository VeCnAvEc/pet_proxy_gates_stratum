use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{warn, Instrument};
use net::server::Server;
use scheduler::scheduler::Scheduler;
use core::job::JobRequest;

pub async fn run_app(
    socket_addr: SocketAddr, semaphore: Arc<Semaphore>,
    tx_cpu_queue_high: Sender<JobRequest>, tx_cpu_queue_norm: Sender<JobRequest>,
    rx_cpu_queue_high: Receiver<JobRequest>, rx_cpu_queue_norm: Receiver<JobRequest>,
    token_shutdown: CancellationToken
) -> anyhow::Result<()> {
    let server = Server::new(
        socket_addr,
        tx_cpu_queue_high,
        tx_cpu_queue_norm,
        token_shutdown.clone()
    ).await?;
    let scheduler = Scheduler::new(
        rx_cpu_queue_high,
        rx_cpu_queue_norm,
        token_shutdown.clone(),
        semaphore
    );
    let mut set = JoinSet::new();

    set.spawn(async move { server.server_run().await }.instrument(tracing::info_span!("server")));
    set.spawn(async move { scheduler.run().await }.instrument(tracing::info_span!("scheduler")));

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