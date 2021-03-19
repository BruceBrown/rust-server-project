use config::{ConfigError, Value};
use config_foundation::{ConfigBuilder, ConfigMetaData, MergedConfig, ServerSettings};
use serde::Deserialize;
/// The config-service extends configuration parsing to include service configuring. The
/// added wrinkle is that we collect the config as a hashmap of enum variants. This allows
/// each service to have its own config, whic h may be similar or distinct from other service
/// configs.
use smart_default::*;
use std::{collections::HashMap, convert::TryFrom, fmt};

/// The config for a server connection
#[derive(Debug, Default, Deserialize, Clone)]
pub struct Server {
    pub port: u16,
    pub url: String,
}

/// The config for a service
#[derive(Debug, Default, Deserialize, Clone)]
pub struct Service {
    pub server: Server,
    pub max_sessions: usize,
}

/// The services. Each variant can have its own config.
#[derive(Debug, SmartDefault, Deserialize, Clone)]
pub enum ServiceConfig {
    #[default]
    EchoService(Service),
    ChatService(Service),
}

impl TryFrom<(String, config::Value)> for ServiceConfig {
    type Error = ConfigError;
    fn try_from((key, value): (String, config::Value)) -> Result<Self, ConfigError> {
        match key {
            _ if key == "EchoService" => {
                let cfg: Service = value.clone().try_into()?;
                Ok(ServiceConfig::EchoService(cfg))
            },
            _ if key == "ChatService" => {
                let cfg: Service = value.clone().try_into()?;
                Ok(ServiceConfig::ChatService(cfg))
            },
            _ => panic!("{}", format!("Need to update TryFrom for ComponentConfig for key={}", key)),
        }
    }
}

impl fmt::Display for ServiceConfig {
    fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // ENV::Development => write!(f, "Development"),
            #[allow(unreachable_patterns)]
            _ => panic!("Need to update Display for ComponentConfig"),
        }
    }
}

/// Settings is root for configuation.
#[derive(Debug, Default, Clone)]
pub struct Settings {
    pub meta_config: ConfigMetaData,
    pub server_config: ServerSettings,
    pub service_config: HashMap<String, ServiceConfig>,
}
impl Settings {
    /// Load the settings
    pub fn load() -> Result<Self, ConfigError> {
        let mut merger = MergedConfig::default();
        let (meta_config, config) = ConfigBuilder::default()
            .with_config_folder_path("../config-service/config/")
            .build(&mut merger)?;
        // try validating all of the service configs
        let services: HashMap<String, Value> = config.get("services")?;
        let mut service_config: HashMap<String, ServiceConfig> = HashMap::new();
        for (key, value) in services {
            let cfg: ServiceConfig = ServiceConfig::try_from((key.clone(), value))?;
            service_config.insert(key, cfg);
        }
        let server_config = config.try_into()?;
        Ok(Self {
            meta_config,
            server_config,
            service_config,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Settings;
    #[test]
    fn test_service_load() {
        match Settings::load() {
            Ok(settings) => println!("settings={:#?}", settings),
            Err(err) => println!("error={:#?}", err),
        }
    }
}
