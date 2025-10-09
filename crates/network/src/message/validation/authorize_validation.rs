use serde_json::Value;
use crate::message::validation::validation::check_obj_on_base_fields;
use crate::message::validation::ValidationError;

pub fn validation_authorize(message: &Value) -> Result<(), ValidationError> {
    let is_exist_base_fields = check_obj_on_base_fields(&message);

    let current_method = "mining.authorize";
    if !is_exist_base_fields {
        return Err(ValidationError::NotFoundBaseFields(current_method.to_string()))
    }

    let params = message.get("params").unwrap();

    if !params.is_array() {
        return Err(ValidationError::ParamsIsNotArray(current_method.to_string()))
    }

    if params.as_array().unwrap().is_empty() {
        return Err(ValidationError::ParamsIsEmpty(current_method.to_string()))
    }

    let params_len = params.as_array().unwrap().len();
    if params_len > 2 {
        return Err(ValidationError::IncorrectNumberOfParameters(params_len.to_string()))
    }

    Ok(())
}