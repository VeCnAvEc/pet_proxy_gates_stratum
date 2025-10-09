use thiserror::Error;
use std::fmt;
pub mod submit_validation;
pub mod authorize_validation;
pub mod subscribe_validation;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Base fields not found: {0}")]
    NotFoundBaseFields(String),
    #[error("Params in {0} is not array")]
    ParamsIsNotArray(String),
    #[error("Array with params is empty: {0}")]
    ParamsIsEmpty(String),
    #[error("Incorrect number of parameters: {0}")]
    IncorrectNumberOfParameters(String)
}

pub mod validation {
    use serde_json::Value;

    pub fn check_obj_on_base_fields(message: &Value) -> bool {
        let is_id = check_id(message);
        let is_method = check_method(message);
        let is_params = check_params(message);

        if is_id && is_method && is_params {
            true
        } else {
            false
        }
    }

    pub fn check_id(obj: &Value) -> bool {
        match obj.get("id") {
            None => {
                false
            }
            Some(_) => {
                true
            }
        }
    }

    pub fn check_method(obj: &Value) -> bool {
        match obj.get("method") {
            None => {
                false
            }
            Some(_) => {
                true
            }
        }
    }

    pub fn check_params(obj: &Value) -> bool {
        match obj.get("params") {
            None => {
                false
            }
            Some(_) => {
                true
            }
        }
    }
}