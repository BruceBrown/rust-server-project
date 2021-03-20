use super::*;
use smart_default::*;

use config::{Config, ConfigError, Source, Value};
use log::{self};
use serde::Deserialize;
use serde_with::*;
use std::collections::{HashMap, HashSet};
use std::fmt;

/// These are some commonly used environment settings. You are free to
/// use them, or not.

/// The Environment enum is required, however the fields can be changed.
/// An environment variable is queried to determine the evironment
/// which the server is running on, that is used to pull in config
/// files matching the environment.
#[derive(Clone, Debug, SmartDefault, Deserialize)]
pub enum Environment {
    Development,
    Testing,
    Stage,
    #[default]
    Production,
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Environment::Development => write!(f, "Development"),
            Environment::Testing => write!(f, "Testing"),
            Environment::Stage => write!(f, "Stage"),
            Environment::Production => write!(f, "Production"),
            #[allow(unreachable_patterns)]
            _ => panic!("Need to update Display for ENV"),
        }
    }
}

impl From<&str> for Environment {
    fn from(env: &str) -> Self {
        match env {
            "Development" => Environment::Development,
            "Testing" => Environment::Testing,
            "Stage" => Environment::Stage,
            "Production" => Environment::Production,
            _ => panic!("Need to update From for Environment"),
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

/// Usually, you'd have your own ServerSettings, but maybe this is good
/// enough for many things.
#[derive(Debug, Default, Deserialize, Clone)]
pub struct ServerSettings {
    pub env: Environment,
    pub log: Log,
    pub server_flavor: String,
    pub features: HashSet<String>,
}

impl ServerSettings {
    pub fn load() -> Result<(ConfigMetaData, Self), ConfigError> {
        let mut merger = MergedConfig::default();
        let config = ConfigBuilder::default().build(&mut merger)?;
        let settings = config.1.try_into()?;
        Ok((config.0, settings))
    }
}

/// This is a custom merger. It merges all of the feature values found in all of the
/// config files. Generally, they'd override each other. This allows for an accumulation
/// instead or replacement.
#[derive(Debug, Default, Clone)]
pub struct MergedConfig {
    features: HashSet<String>,
}

impl ConfigMerger for MergedConfig {
    fn merge_from(&mut self, config: &Config) {
        // merge in new features
        let features: Vec<String> = config.get("features").unwrap_or_default();
        for f in features {
            self.features.insert(f);
        }
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
        let vec = Value::from(vec);
        map.insert("features".to_string(), vec);
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let res = ServerSettings::load();
        if let Err(err) = res {
            println!("error={:#?}", err);
        } else if let Ok((meta_config, settings)) = res {
            println!("meta_config={:#?} features={:#?}", meta_config, settings.features);
        }
    }
}
