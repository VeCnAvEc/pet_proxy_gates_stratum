use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use tokio_util::sync::CancellationToken;
use crate::app::supervisor::run_app;
use crate::domain::job::{JobRequest};
use crate::logs::logs::init_logs;
use crate::network::server::Server;
use crate::scheduler::Scheduler;

mod logs;
mod network;
mod methods;
mod domain;
mod scheduler;
mod app;

#[tokio::main(flavor="multi_thread")]
async fn main() {
    init_logs();

    let host = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let port = 5555;

    let cpu_limiter = Arc::new(Semaphore::new(10));

    let (tx_cpu_queue_high, rx_cpu_queue_high) = mpsc::channel::<JobRequest>(256);
    let (tx_cpu_queue_norm, rx_cpu_queue_norm) = mpsc::channel::<JobRequest>(256);

    let token = CancellationToken::new();

    let socket_addr = SocketAddr::new(host, port);

    let server = Server::new(
        socket_addr,
        tx_cpu_queue_high,
        tx_cpu_queue_norm,
        token.clone()
    ).await;
    let scheduler = Scheduler::new(
        rx_cpu_queue_high,
        rx_cpu_queue_norm,
        token.clone(),
        cpu_limiter
    );

    let result = run_app(server.unwrap(), scheduler, token).await;
}
