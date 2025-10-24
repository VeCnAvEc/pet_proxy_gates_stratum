use std::env;
use std::fs::File;
use serde_json::from_reader;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub stratum_host: String,
    pub stratum_port: u16,
    pub database: DatabaseConfig,
    pub api_key: String,
    pub api_url: String
}

#[derive(Debug, Deserialize)]
struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub db_name: String,
    pub password: String,
    pub connections_limit: u16
}

impl Config {
    pub fn new() -> Config {
        let default_path = "./config/config.json";
        let args: Vec<String> = env::args().collect();

        let config_path = if args.len() < 2 {
            default_path
        } else {
            args[1].as_str()
        };

        let file = File::open(config_path).expect("The file couldn't be opened");

        let reader = std::io::BufReader::new(file);
        let config: Config = from_reader(reader).expect("Couldn't read JSON");

        config
    }
}