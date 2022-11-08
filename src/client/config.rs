use std::{fs, io};

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub bind_host: String,
    pub bind_port: u16,
    pub salt: String,
    pub remote_host: String,
    pub remote_port: u16,
    pub username: String,
    pub password: String,
    pub allow_insecure: bool,
    pub http_path: String,
    pub http_domain: String,
    pub http_user_agent: String,
}

pub static INSTANCE: OnceCell<Config> = OnceCell::new();

impl Config {
    pub fn global() -> &'static Config {
        INSTANCE.get().expect("Config is not initialized")
    }

    pub fn from_cli(path: &String) -> Result<Config, io::Error> {
        let content = fs::read_to_string(path)?;
        // let yaml_str = include_str!("../../app.yaml");
        let config: Config = serde_yaml::from_str(&content)
            .map_err(|_err| io::Error::new(io::ErrorKind::Other, _err))?;
        Ok(config)
    }
}

pub fn init(path: &String) -> Result<(), io::Error> {
    let conf = Config::from_cli(path)?;
    INSTANCE.set(conf).unwrap();
    Ok(())
}
