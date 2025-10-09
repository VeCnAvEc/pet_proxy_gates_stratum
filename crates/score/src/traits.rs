use serde_json::Value;

use thiserror::Error;
use crate::utils::parse_params_array;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("expected object with `params` array")]
    NoParamsArray,
    #[error("missing param at index {0}")]
    MissingParam(usize),
    #[error("invalid type for param {0}: expected string-like")]
    InvalidType(usize),
    #[error("other: {0}")]
    Other(String),
}


/// Set of functions for parsing some the methods (mining_authorize, mining_subscribe, mining_submit)
pub trait FromParams: Sized {
    fn from_params(params: &[Value]) -> Result<Self, ParseError>;

    fn from_value(v: &Value) -> Result<Self, ParseError> {
        let params = parse_params_array(v)?;
        Self::from_params(params)
    }
}

pub fn extract_params_array(v: &Value) -> Result<&Vec<Value>, String> {
    v.get("params")
        .and_then(|p| p.as_array())
        .ok_or_else(|| "Missing or invalid 'params' field".to_string())
}