use std::{fs, io};

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub bind_host: String,
    pub bind_port: u16,
    pub salt: String,

    pub cert_path: String,
    pub key_path: String,
    pub sni: String,
    pub use_ssl: bool,

    pub users: HashMap<String, String>,
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
