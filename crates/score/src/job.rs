use std::borrow::Cow;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{Mutex, oneshot};
use crate::miner::Miner;
use crate::traits::{extract_params_array, FromParams, ParseError};

#[derive(Debug)]
pub enum Job {
    MiningSubmit((SubmitParams, Arc<Mutex<Miner>>)), // mining.submit
    MiningAuthorize((AuthorizeParams, Arc<Mutex<Miner>>)), // mining.authorize
    MiningSubscribe((SubscribeParams, Arc<Mutex<Miner>>)), // mining.subscribe
    Ping,
}

#[derive(Debug)]
pub enum ProxyMessage<'a> {
    Wait,
    Request(Cow<'a, str>),
    Response(Cow<'a, str>),
    Err(Cow<'a, str>)
}

#[derive(Debug)]
pub struct JobRequest {
    pub job: Job, // mining method
    pub respond_to: oneshot::Sender<ProxyMessage<'static>>, // a channel for responding to the user
    // pub miner_notify_rx: Option<tokio::sync::mpsc::Receiver<String>> // If the JobRequest is the mining.notify we will need to open a stream for the message flow
}

#[derive(Debug, Deserialize)]
pub struct SubmitParams {
    pub workername: String, // worker_name ASIC
    pub job_id: String, // job_id from mining.notify
    pub extranonce2: String, // user extranonce
    pub n_time: String, // n_time in format a little inding
    pub nonce: String, // miner nonce
    pub n_bits: Option<String> // changed bytes
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthorizeParams {
    username: String, // user's username on pool
    password: Option<String> // pool pass or preferred diff
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubscribeParams {
    agent_version: String, // example: user agent/version
    extranonce1: Option<String> // Optional extranonce1. If miner wants to continue with his past extranonce1
}

// Build a structure submit from params
impl FromParams for SubmitParams {
    fn from_params(params: &[Value]) -> Result<Self, ParseError> {
        Ok(
            SubmitParams {
                workername: params.get(0).and_then(|worker| Some(worker.as_str().unwrap().to_string())).unwrap(),
                job_id: params.get(1).and_then(|job_id| Some(job_id.as_str().unwrap().to_string())).unwrap(),
                extranonce2: params.get(2).and_then(|extranonce2| Some(extranonce2.as_str().unwrap().to_string())).unwrap(),
                n_time: params.get(3).and_then(|n_time| Some(n_time.as_str().unwrap().to_string())).unwrap(),
                nonce: params.get(4).and_then(|nonce| Some(nonce.as_str().unwrap().to_string())).unwrap(),
                n_bits: Some(params.get(5).and_then(|n_bits| Some(n_bits.as_str().unwrap().to_string())).or(Some("000000".to_string())).unwrap()),
            }
        )
    }
}

// Build a structure authorize from params
impl FromParams for AuthorizeParams {
    fn from_params(params: &[Value]) -> Result<Self, ParseError> {
        Ok(
            AuthorizeParams {
                username: params.get(0).and_then(|username| Some(username.as_str().unwrap().to_string())).unwrap(),
                password: params.get(1).and_then(|pass| Some(pass.as_str().unwrap().to_string())),
            }
        )
    }
}

// Build a structure subscribe from params
impl FromParams for SubscribeParams {
    fn from_params(params: &[Value]) -> Result<Self, ParseError> {
        Ok(
            SubscribeParams {
                agent_version: params.get(0).and_then(|text| Some(text.as_str().unwrap().to_string())).unwrap_or("Unknown".to_string()),
                extranonce1: params.get(1).and_then(|extranonce1| Some(extranonce1.as_str().unwrap().to_string())),
            }
        )
    }
}

impl SubmitParams {
    pub fn from_value(v: &Value) -> Result<Self, ParseError> {
        let params = extract_params_array(v);
        if let Err(err) = params {
            return Err(ParseError::Other(err));
        }
        SubmitParams::from_params(params.unwrap())
    }
}

impl AuthorizeParams {
    pub fn form_value(v: &Value) -> Result<Self, ParseError> {
        let params = extract_params_array(v);
        if let Err(err) = params {
            return Err(ParseError::Other(err));
        }
        AuthorizeParams::from_params(params.unwrap())
    }
}

impl SubscribeParams {
    pub fn from_value(v: &Value) -> Result<Self, ParseError> {
        let params = extract_params_array(v);
        if let Err(err) = params {
            return Err(ParseError::Other(err));
        }

        SubscribeParams::from_params(params.unwrap())
    }
}

impl AuthorizeParams {
    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &Option<String> {
        &self.password
    }
}