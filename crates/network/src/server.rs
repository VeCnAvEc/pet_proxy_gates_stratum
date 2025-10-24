use std::net::SocketAddr;
use std::sync::{Arc, atomic::{AtomicU64}};
use std::sync::atomic::Ordering;

use dashmap::DashMap;

use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::sync::mpsc::{Sender};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use tracing::{error, info, warn};
use config::Config;
use score::job::JobRequest;

use crate::connection::handle_connection;

static TOTAL_CONN: AtomicU64 = AtomicU64::new(0);

pub type ConnId = u64;

struct ConnHandle {
    token: CancellationToken,
    join: JoinHandle<()>
}

#[derive(Clone)]
pub struct Server {
    listener: Arc<TcpListener>,
    shutdown: CancellationToken,
    tx_queue_high: Sender<JobRequest>,
    tx_queue_norm: Sender<JobRequest>,
    conns: Arc<DashMap<ConnId, ConnHandle>>,
    config: Arc<Config>
}

impl Server {
    pub async fn new(
        addr: SocketAddr, tx_queue_high: Sender<JobRequest>,
        tx_queue_norm: Sender<JobRequest>, token: CancellationToken,
        config: Arc<Config>
    ) -> anyhow::Result<Server> {
        let listener = Arc::new(TcpListener::bind(addr).await?);
        let conns = Arc::new(DashMap::new());

        Ok(Server {
            listener,
            shutdown: token,
            tx_queue_high,
            tx_queue_norm,
            conns,
            config
        })
    }
    pub async fn server_run(self) -> anyhow::Result<()> {
        let mut next_id: ConnId = 0;

        loop {
            select! {
                _ = self.shutdown.cancelled() => break,

                sock = self.listener.accept() => {
                    let (socket, addr) = match sock {
                        Ok((socket, addr)) => (socket, addr),
                        Err(err) => {
                            error!("Socket connection error: {}", err.to_string());
                            continue;
                        }
                    };
                    next_id += 1;
                    let id = next_id;

                    self.spawn_conn(id, addr, socket).await;
                }

            }
        }

        Ok(())
    }

    async fn spawn_conn(&self, conn_id: ConnId, addr: SocketAddr, socket: TcpStream) {
        let token = self.shutdown.child_token();
        let token_clone = token.clone();
        let token_handle_connection = token.clone();

        let tx_high = self.tx_queue_high.clone();
        let tx_norm = self.tx_queue_norm.clone();
        let conn = self.conns.clone();

        let api_url = self.config.api_url.clone();
        let join = tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, token_handle_connection, conn_id, tx_high, tx_norm, api_url).await {
                warn!(%addr, %conn_id, error=?e, "conn error")
            }
        });

        info!(%addr, %conn_id, "A new connection");
        TOTAL_CONN.fetch_add(1, Ordering::Relaxed);

        let conn_handle = ConnHandle {
            token: token_clone,
            join,
        };

        conn.insert(conn_id, conn_handle);
    }
}

