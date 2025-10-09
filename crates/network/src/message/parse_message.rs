use serde_json::{from_str, Value};
use tracing::{info, warn};
use score::job::{Authorize, Submit, Subscribe};
use crate::message::{Command};
use crate::message::validation::authorize_validation::validation_authorize;
use crate::message::validation::submit_validation::submit_validation;
use crate::message::validation::subscribe_validation::validation_subscribe;

pub fn parse_message(line: &str) -> anyhow::Result<Command> {
    let message_json = from_str::<Value>(line);

    if let Err(_) = message_json {
        return Ok(Command::Unknown)
    }
    let message_json = message_json?;

    let method = message_json.get("method").unwrap();

    match method.as_str().unwrap() {
        "mining.submit" => {
            let validation_result = submit_validation(&message_json);
            if let Err(err) = validation_result {
                warn!("Validation Error: {:?}", err);
                return Ok(Command::Unknown);
            }
            let submit = Submit::from_value(&message_json)?;

            Ok(Command::CSubmit(submit))
        },
        "mining.authorize" => {
            let validation_result = validation_authorize(&message_json);
            if let Err(err) = validation_result {
                warn!("Validation Error: {:?}", err);
                return Ok(Command::Unknown)
            }
            let authorize = Authorize::form_value(&message_json)?;

            Ok(Command::CAuthorize(authorize))
        },
        "mining.subscribe" => {Ok(Command::Unknown)}
        _ => {
            let validation_result = validation_subscribe(&message_json);
            if let Err(err) = validation_result {
                warn!("Validation Error: {:?}", err);
                return Ok(Command::Unknown)
            }
            let subscribe = Subscribe::from_value(&message_json)?;

            Ok(Command::CSubscribe(subscribe))
        }
    }
}
