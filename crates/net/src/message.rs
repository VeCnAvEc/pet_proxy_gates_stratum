use core::job::Submit;

#[derive(Debug)]
pub enum Command {
    Ping,
    CSubmit(Submit),
    Unknown
}

pub fn parse_message(line: &str) -> anyhow::Result<Command> {
    match line {
        "PING" => Ok(Command::Ping),
        "mining.submit" => {
            let test_submit = Submit {
                worker_name: "".to_string(),
                job_id: "".to_string(),
                extranonce2: "".to_string(),
                n_time: "".to_string(),
                nonce: "".to_string(),
            };

            Ok(Command::CSubmit(test_submit))
        }
        _ => Ok(Command::Unknown)
    }
}
