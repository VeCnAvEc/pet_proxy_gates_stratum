use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{warn, Instrument};
use crate::network::server::Server;
use crate::scheduler::Scheduler;

pub async fn run_app(
    server: Server, sched: Scheduler,
    shutdown: CancellationToken
) -> anyhow::Result<()> {
    let mut set = JoinSet::new();

    set.spawn(async move { server.server_run().await }.instrument(tracing::info_span!("server")));
    set.spawn(async move { sched.run().await }.instrument(tracing::info_span!("scheduler")));

    tokio::select! {
        _ = shutdown.cancelled() => {}
        res = set.join_next() => {
            match set.join_next().await {
                Some(Ok(_)) => { }
                Some(Err(e)) => return Err(e.into()),
                None => { }
            }
        }
    }
    shutdown.cancel();

    while let Some(res) = set.join_next().await {
        if let Err(e) = res { warn!(?e, "task error") }
    }

    Ok(())
}