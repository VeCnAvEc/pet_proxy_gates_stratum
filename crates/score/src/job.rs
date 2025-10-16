use std::sync::Arc;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::{Mutex, oneshot};
use crate::miner::Miner;
use crate::traits::{extract_params_array, FromParams, ParseError};

#[derive(Debug)]
pub enum Job {
    MiningSubmit((Submit, Arc<Mutex<Miner>>)), // mining.submit
    MiningAuthorize((Authorize, Arc<Mutex<Miner>>)), // mining.authorize
    MiningSubscribe((Subscribe, Arc<Mutex<Miner>>)), // mining.subscribe
    Ping,
}

#[derive(Debug)]
pub struct JobRequest {
    pub job: Job, // mining method
    pub respond_to: oneshot::Sender<&'static str>, // a channel for responding to the user
}

#[derive(Debug, Deserialize)]
pub struct Submit {
    pub workername: String, // worker_name ASIC
    pub job_id: String, // job_id from mining.notify
    pub extranonce2: String, // user extranonce
    pub n_time: String, // n_time in format a little inding
    pub nonce: String, // miner nonce
    pub n_bits: Option<String> // changed bytes
}

#[derive(Debug)]
pub struct Authorize {
    username: String, // user's username on pool
    password: Option<String> // pool pass or preferred diff
}

#[derive(Debug, Deserialize)]
pub struct Subscribe {
    agent_version: String, // example: user agent/version
    extranonce1: Option<String> // Optional extranonce1. If miner wants to continue with his past extranonce1
}

// Build a structure submit from params
impl FromParams for Submit {
    fn from_params(params: &[Value]) -> Result<Self, ParseError> {
        Ok(
            Submit {
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
impl FromParams for Authorize {
    fn from_params(params: &[Value]) -> Result<Self, ParseError> {
        Ok(
            Authorize {
                username: params.get(0).and_then(|username| Some(username.as_str().unwrap().to_string())).unwrap(),
                password: params.get(1).and_then(|pass| Some(pass.as_str().unwrap().to_string())),
            }
        )
    }
}

// Build a structure subscribe from params
impl FromParams for Subscribe {
    fn from_params(params: &[Value]) -> Result<Self, ParseError> {
        Ok(
            Subscribe {
                agent_version: params.get(0).and_then(|text| Some(text.as_str().unwrap().to_string())).unwrap_or("Unknown".to_string()),
                extranonce1: params.get(1).and_then(|extranonce1| Some(extranonce1.as_str().unwrap().to_string())),
            }
        )
    }
}

impl Submit {
    pub fn from_value(v: &Value) -> Result<Self, ParseError> {
        let params = extract_params_array(v);
        if let Err(err) = params {
            return Err(ParseError::Other(err));
        }
        Submit::from_params(params.unwrap())
    }
}

impl Authorize {
    pub fn form_value(v: &Value) -> Result<Self, ParseError> {
        let params = extract_params_array(v);
        if let Err(err) = params {
            return Err(ParseError::Other(err));
        }
        Authorize::from_params(params.unwrap())
    }
}

impl Subscribe {
    pub fn from_value(v: &Value) -> Result<Self, ParseError> {
        let params = extract_params_array(v);
        if let Err(err) = params {
            return Err(ParseError::Other(err));
        }

        Subscribe::from_params(params.unwrap())
    }
}