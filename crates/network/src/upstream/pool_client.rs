use std::net::{IpAddr, SocketAddr};
use std::string::ParseError;
use std::sync::Arc;
use std::time::Duration;
use anyhow::{anyhow, Context};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, mpsc};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

pub struct PoolClient {
    miner_channel_writer: mpsc::Sender<String>,
    tasks: Vec<JoinHandle<()>>
}

impl PoolClient {
    pub async fn new(pool_address: &String, up_to_miner: mpsc::Sender<String>) -> anyhow::Result<Self> {
        // let (host, port) = match pool_address.split_once(":") {
        //     Some((host, port)) => {
        //         match port.parse::<u16>() {
        //             Ok(port) => Ok((host.to_string(), port)),
        //             Err(err) => Err(anyhow!("ParseError: {:?}", err))
        //         }
        //     }
        //     None => Err(anyhow!("ParseError with this is pool_address: {}", pool_address)),
        // }?;
        // info!("new pool client 2");
        // info!("new pool client host: {}", host);
        //
        // let host = IpAddr::V4(host.parse()?);
        //
        // let socket_address = SocketAddr::new(host, port);

        info!("PRE CONN");
        let stream = tokio::time::timeout(Duration::from_secs(3), TcpStream::connect(pool_address))
            .await
            .context("connect timeout")??;
        info!("CONN");

        let (read_half, mut write_half) = tokio::io::split(stream);
        let mut reader = BufReader::new(read_half);

        let (miner_tx, mut miner_rx) = mpsc::channel(32);

        let writer_handle = tokio::spawn(async move {
            info!("Writer handle to pool started!");
            while let Some(msg) = miner_rx.recv().await {
                let mut to_write: String = msg;
                info!("PoolClient msg -> {}", to_write); // workFlow2.asc6
                if !to_write.ends_with('\n') {
                    to_write.push_str("\n");
                }
                if let Err(e) = write_half.write_all(to_write.as_bytes()).await {
                    error!("Error writing to upstream: {:?}", e);
                    break;
                }
                info!("miner->pool writer exiting");
            }
        });

        let reader_handle = tokio::spawn(async move {
            info!("Reader handle from pool to started!");
            let mut line = String::new();
            loop {
                line.clear();
                let n = match reader.read_line(&mut line).await {
                    Ok(0) => {
                        info!("upstream closed connection");
                        break;
                    }
                    Ok(n) => {
                        info!("{} bytes were received", n);
                        n
                    },
                    Err(e) => {
                        error!("error reading from upstream: {:?}", e);
                        break
                    }
                };

                let s = if line.ends_with('\n') {
                    line.trim_end_matches('\n').to_string()
                } else {
                    line.clone()
                };

                info!("Response from pool -> {}", s);

                if let Err(_e) = up_to_miner.send(s).await {
                    warn!("miner receiver dropped, stopping reading from upstream");
                }
            }
        });

        Ok(Self {
            miner_channel_writer: miner_tx,
            tasks: vec![writer_handle, reader_handle]
        })
    }

    pub fn miner_channel_writer(&self) -> mpsc::Sender<String> {
        self.miner_channel_writer.clone()
    }

    pub async fn shutdown(self) {
        for t in self.tasks {
            t.abort();
        }
    }
}