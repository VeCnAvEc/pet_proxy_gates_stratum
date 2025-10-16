use std::net::{IpAddr, SocketAddr};

use uuid::Uuid;

#[derive(Debug)]
pub struct Miner {
    miner_id: Uuid,
    miner_host: IpAddr,
    miner_port: u16,
    time_authorize: Option<u64>,
    pool_addr: String,
    share_count: u64,
    miner_diff: u64,
    worker_name: String,
    is_subscribe: bool,
    is_authorize: bool
}

impl Miner {
    pub fn new(socket_address: SocketAddr) -> Self {
        let host = socket_address.ip();
        let port = socket_address.port();

        Self {
            miner_id: Uuid::new_v4(),
            miner_host: host,
            miner_port: port,
            time_authorize: None,
            // Need do default pool address for example via.btc.com:3333
            pool_addr: "".to_string(),
            share_count: 0,
            miner_diff: 0,
            worker_name: "".to_string(),
            is_subscribe: false,
            is_authorize: false,
        }
    }

    // --- SETTERS ---
    pub fn set_time_authorize(&mut self, time: u64) {
        self.time_authorize = Some(time);
    }

    pub fn set_pool_addr(&mut self, pool_addr: String) {
        self.pool_addr = pool_addr;
    }

    pub fn set_share_count(&mut self, count: u64) {
        self.share_count = count;
    }

    pub fn increment_share_count(&mut self) {
        self.share_count += 1;
    }

    pub fn set_miner_diff(&mut self, diff: u64) {
        self.miner_diff = diff;
    }

    pub fn set_worker_name<S: Into<String>>(&mut self, name: S) {
        self.worker_name = name.into();
    }

    pub fn set_is_subscribe(&mut self, value: bool) {
        self.is_subscribe = value;
    }

    pub fn set_is_authorize(&mut self, value: bool) {
        self.is_authorize = value;
    }

    // --- GETTERS ---

    pub fn miner_id(&self) -> Uuid {
        self.miner_id
    }

    pub fn miner_host(&self) -> IpAddr {
        self.miner_host
    }

    pub fn miner_port(&self) -> u16 {
        self.miner_port
    }

    pub fn time_authorize(&self) -> Option<u64> {
        self.time_authorize
    }

    pub fn pool_addr(&self) -> &str {
        &self.pool_addr
    }

    pub fn share_count(&self) -> u64 {
        self.share_count
    }

    pub fn miner_diff(&self) -> u64 {
        self.miner_diff
    }

    pub fn worker_name(&self) -> &str {
        &self.worker_name
    }

    pub fn is_subscribe(&self) -> bool {
        self.is_subscribe
    }

    pub fn is_authorize(&self) -> bool {
        self.is_authorize
    }
}