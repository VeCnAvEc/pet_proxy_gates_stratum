use serde_json::Value;
use crate::traits::ParseError;

pub fn get_param_as_string(params: &[Value], idx: usize) -> Result<String, ParseError> {
    params.get(idx)
        .ok_or(ParseError::MissingParam(idx))
        .and_then(|val| {
            match val {
                Value::String(s) => Ok(s.clone()),
                Value::Number(n) => Ok(n.to_string()),
                Value::Bool(b) => Ok(b.to_string()),
                _ => Err(ParseError::InvalidType(idx)),
            }
        })
}

pub fn opt_param_as_string(params: &[Value], idx: usize) -> Result<Option<String>, ParseError> {
    match params.get(idx) {
        None => Ok(None),
        Some(Value::Null) => Ok(None),
        Some(v) => {
            match v {
                Value::String(s) => Ok(Some(s.clone())),
                Value::Number(n) => Ok(Some(n.to_string())),
                Value::Bool(b) => Ok(Some(b.to_string())),
                _ => Err(ParseError::InvalidType(idx)),
            }
        }
    }
}

pub fn parse_params_array(v: &Value) -> Result<&Vec<Value>, ParseError> {
    v.get("params")
        .and_then(|p| p.as_array())
        .ok_or(ParseError::NoParamsArray)
}