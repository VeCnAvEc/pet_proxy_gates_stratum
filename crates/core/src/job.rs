use futures::channel::oneshot;

#[derive(Debug)]
pub enum Job {
    MiningSubmit(Submit),
    Ping,
}

#[derive(Debug)]
pub struct JobRequest {
    pub job: Job,
    pub respond_to: oneshot::Sender<&'static str>,
}

#[derive(Debug)]
pub struct Submit {
    pub worker_name: String,
    pub job_id: String,
    pub extranonce2: String,
    pub n_time: String,
    pub nonce: String
}