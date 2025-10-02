use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;

pub enum Outcome {
    Replied,
    NoReply,
    Cancelled,
    IoError(tokio::io::Error)
}

pub async fn await_and_replay(
    socket: &mut TcpStream,
    rx: &mut tokio::sync::oneshot::Receiver<&'static str>,
    cancel: CancellationToken
) -> Outcome {
    tokio::select! {
        _ = cancel.cancelled() => {
            Outcome::Cancelled
        }
        res = rx => {
            match res {
                Ok(msg) => {
                    if let Err(err) = socket.write_all(msg.as_bytes()).await {
                        Outcome::IoError(err)
                    } else {
                        Outcome::Replied
                    }
                }
                Err(err) => { Outcome::NoReply }
            }
        }
    }
}