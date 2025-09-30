use tokio::sync::{mpsc::Sender, oneshot};

use crate::methods::submit::Submit;

#[derive(Debug)]
pub(crate) enum Job {
    MiningSubmit(Submit),
    Ping,
}

#[derive(Debug)]
pub struct JobRequest {
    pub job: Job,
    pub respond_to: oneshot::Sender<&'static str>,
}

impl JobRequest {
    pub async fn send_request_job(self, tx: Sender<JobRequest>) {
        tx.send(self).await.unwrap()
    }
}