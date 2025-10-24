use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use tokio::sync::{mpsc, Semaphore};
use tokio_util::sync::CancellationToken;

use app::supervisor::run_app;
use score::job::JobRequest;
use utils::logs::init_logs;

#[tokio::main(flavor="multi_thread")]
async fn main() {
    init_logs();

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 5555);

    let semaphore = Arc::new(Semaphore::new(100));
    let (tx_cpu_queue_high, rx_cpu_queue_high) = mpsc::channel::<JobRequest>(256);
    let (tx_cpu_queue_norm, rx_cpu_queue_norm) = mpsc::channel::<JobRequest>(256);
    let cancel = CancellationToken::new();

    if let Err(e) = run_app(addr, semaphore, tx_cpu_queue_high, tx_cpu_queue_norm, rx_cpu_queue_high, rx_cpu_queue_norm, cancel).await {
        eprintln!("Application error: {e:?}");
    }
}