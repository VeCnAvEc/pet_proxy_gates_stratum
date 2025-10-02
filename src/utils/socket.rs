use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;

pub async fn await_and_replay(
    socket: &mut TcpStream,
    rx: &mut tokio::sync::oneshot::Receiver<&'static str>,
    cancel: CancellationToken
) -> anyhow::Result<()> {
    tokio::select! {
        _ = cancel.cancelled() => {}
        res = rx => {
            if let Ok(msg) = res {
                socket.write_all(msg.as_bytes()).await?;
            }
        }
    }

    Ok(())
}