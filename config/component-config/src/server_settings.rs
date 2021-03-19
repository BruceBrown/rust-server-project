use super::*;
use smart_default::*;

use config::{Config, ConfigError, Source, Value};
use log::{self};
use serde::Deserialize;
use serde_with::*;
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::fmt;

#[derive(Debug, Default, Clone)]
pub struct Settings {
    pub meta_config: MetaConfig,
    pub server_config: ServerConfig,
    pub component_config: HashMap<String, ComponentConfig>,
}
impl Settings {
    pub fn load() -> Result<Self, ConfigError> {
        let mut merger = MergedConfig::default();
        let (meta_config, config) = ConfigBuilder::default().build(&mut merger)?;
        // try validating all of the component configs
        let components: HashMap<String, Value> = config.get("services")?;
        let mut component_config: HashMap<String, ComponentConfig> = HashMap::new();
        for (key, value) in components {
            let cfg: ComponentConfig = ComponentConfig::try_from((key.clone(), value))?;
            component_config.insert(key, cfg);
        }
        let server_config = config.try_into()?;
        Ok(Self {
            meta_config,
            server_config,
            component_config,
        })
    }
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct ServerConfig {
    pub env: ENV,
    pub log: Log,
    pub server_flavor: String,
    pub features: HashSet<String>,
}

/// The ENV enum is required, however the fields can be changed.
/// An environment variable is queried to determine the evironment
/// which the server is running on, that is used to pull in config
/// files matching the environment.
#[derive(Clone, Debug, SmartDefault, Deserialize)]
pub enum ENV {
    Development,
    Testing,
    Stage,
    #[default]
    Production,
}

impl fmt::Display for ENV {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ENV::Development => write!(f, "Development"),
            ENV::Testing => write!(f, "Testing"),
            ENV::Stage => write!(f, "Stage"),
            ENV::Production => write!(f, "Production"),
            #[allow(unreachable_patterns)]
            _ => panic!("Need to update Display for ENV"),
        }
    }
}

impl From<&str> for ENV {
    fn from(env: &str) -> Self {
        match env {
            "Development" => ENV::Development,
            "Testing" => ENV::Testing,
            "Stage" => ENV::Stage,
            "Production" => ENV::Production,
            _ => panic!("Need to update From for ENV"),
        }
    }
}

/// This is the log filter. Notice that we're able to use the Display and FromStr impl.
/// Normally, we'd have to use From<&str>, but serde_as has our back and provides a means
/// to use Display and FromStr
#[serde_as]
#[derive(Debug, Deserialize, SmartDefault, Copy, Clone)]
pub struct Log {
    #[serde_as(as = "DisplayFromStr")]
    #[default(log::LevelFilter::Warn)]
    pub level: log::LevelFilter,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct Server {
    pub port: u16,
    pub url: String,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct EchoService {
    pub server: Server,
    pub max_sessions: usize,
}

#[derive(Debug, SmartDefault, Deserialize, Clone)]
pub enum ComponentConfig {
    #[default]
    EchoService(EchoService),
    ChatService(EchoService),
}

impl TryFrom<(String, config::Value)> for ComponentConfig {
    type Error = ConfigError;
    fn try_from((key, value): (String, config::Value)) -> Result<Self, ConfigError> {
        match key {
            _ if key == "EchoService" => {
                let cfg: EchoService = value.clone().try_into()?;
                Ok(ComponentConfig::EchoService(cfg))
            },
            _ if key == "ChatService" => {
                let cfg: EchoService = value.clone().try_into()?;
                Ok(ComponentConfig::ChatService(cfg))
            },
            _ => panic!("{}", format!("Need to update TryFrom for ComponentConfig for key={}", key)),
        }
    }
}

impl fmt::Display for ComponentConfig {
    fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // ENV::Development => write!(f, "Development"),
            #[allow(unreachable_patterns)]
            _ => panic!("Need to update Display for ComponentConfig"),
        }
    }
}

// Merged config is called after each config is merged, allowing it to perform custom
// merging of fields. As one of the last steps, the Merged config is merged, which
// overrides with the custom merge. This is primarily intended for fields, such as
// features, which is a sequence scattered over several config files. This provides
// the gather point.
#[derive(Debug, Default, Clone)]
struct MergedConfig {
    features: HashSet<String>,
}

impl ConfigMerger for MergedConfig {
    fn merge_from(&mut self, config: &Config) {
        // merge in new features
        let features: Vec<String> = config.get("features").unwrap_or_default();
        features.iter().for_each(|f| {
            self.features.insert(f.to_string());
        });
    }
    fn merge_into(&mut self, config: &mut Config) -> Result<(), ConfigError> {
        match config.merge(self.clone()) {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }
}

impl config::Source for MergedConfig {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> { Box::new((*self).clone()) }

    /// Collect all configuration properties available from this source and return
    /// a HashMap.
    fn collect(&self) -> Result<HashMap<String, Value>, ConfigError> {
        let mut map: HashMap<String, Value> = HashMap::new();
        let vec: Vec<Value> = self.features.iter().map(|f| Value::from(f.clone())).collect();
        let value = Value::from(vec);
        map.insert("features".to_string(), value);
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let res = Settings::load();
        if let Err(err) = res {
            println!("error={:#?}", err);
        } else if let Ok(settings) = res {
            println!("settings={:#?} ", settings);
        }
    }
}
