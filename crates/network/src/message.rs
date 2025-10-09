pub mod parse_message;
mod validation;

use score::job::{Submit, Authorize, Subscribe};

#[derive(Debug)]
pub enum Command {
    Ping,
    CSubmit(Submit),
    CAuthorize(Authorize),
    CSubscribe(Subscribe),
    Unknown
}