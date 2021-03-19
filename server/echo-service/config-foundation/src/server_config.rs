use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

/// The ConfigMerger is passed into settings and provides custom handling of config
/// fields. After each config file is merged, the merge_from() method is called which
/// can perform customized config merging. As a final step in creating settings,
/// the Source is merged via the merge_into() method.
pub trait ConfigMerger {
    fn merge_from(&mut self, config: &Config);
    fn merge_into(&mut self, config: &mut Config) -> Result<(), ConfigError>;
}

/// The env var for determining the operating environment.
const CONFIG_ENV_VAR: &str = "RUN_ENV";
/// The default operating environment.
const CONFIG_ENV_DEFAULT: &str = "Production";
/// The prefix for patching the Environment.
const CONFIG_ENV_PREFIX: &str = "ea";
/// The prefix for patching the Environment.
const CONFIG_ENV_SEPARATOR: &str = "__";
/// The path to config root.
const CONFIG_FOLDER_PATH: &str = "./config/";
/// The suffix for .toml files.
const CONFIG_TOML_SUFFIX: &str = ".toml";
/// The suffix for .json files.
const CONFIG_JSON_SUFFIX: &str = ".json";

/// The config for server personality, this is is treated as a named folder under config
/// and contains default and environment depended config. It is pulled from the config
/// after processing default and ENV configuration.
const CONFIG_ENV_VAR_SERVER_FLAVOR: &str = "SERVER_FLAVOR";
/// The default flavor.
const CONFIG_ENV_VAR_SERVER_FLAVOR_DEFAULT: &str = "";
/// The name of the default configuration.
const CONFIG_DEFAULT_NAME: &str = "default";

/// The ConfigBuilder provides a default set of config parameter, which are used
/// in locating config information in files and the environment. It also provides a
/// means of setting individual fields.
#[derive(Debug, Default)]
pub struct ConfigBuilder {
    config: ConfigMetaData,
}

#[allow(dead_code)]
impl ConfigBuilder {
    /// Override for config_env_var
    pub fn with_config_env_var(mut self, val: &str) -> Self {
        self.config.config_env_var = val.to_string();
        self
    }

    /// Override for config_env_prefix
    pub fn with_config_env_prefix(mut self, val: &str) -> Self {
        self.config.config_env_prefix = val.to_string();
        self
    }

    /// Override for config_env_separator
    pub fn with_config_env_separator(mut self, val: &str) -> Self {
        self.config.config_env_separator = val.to_string();
        self
    }

    /// Override for config_folder_path
    pub fn with_config_folder_path(mut self, val: &str) -> Self {
        self.config.config_folder_path = val.to_string();
        self
    }

    /// Override for config_toml_suffix
    pub fn with_config_toml_suffix(mut self, val: &str) -> Self {
        self.config.config_toml_suffix = val.to_string();
        self
    }

    /// Override for config_json_suffix
    pub fn with_config_json_suffix(mut self, val: &str) -> Self {
        self.config.config_json_suffix = val.to_string();
        self
    }

    /// Override for config_default_name
    pub fn with_config_default_name(mut self, val: &str) -> Self {
        self.config.config_default_name = val.to_string();
        self
    }

    /// Build the Config database, returing it, along with the meta environment used to
    /// produce it.
    pub fn build(&self, merger: &mut dyn ConfigMerger) -> Result<(ConfigMetaData, Config), ConfigError> {
        match self.create(merger) {
            Ok(config) => Ok((self.config.clone(), config)),
            Err(err) => Err(err),
        }
    }

    /// Given a configuration, and a file path, along with a merger, this will attempt to merge the toml and json
    /// files into the configuration. Additionally, it will pass the merged config into the merger, where
    /// custom merging can be performed.
    fn merge_filepath(&self, config: &mut Config, file_path: &str, merger: &mut dyn ConfigMerger) -> Result<(), ConfigError> {
        let config_path = format!("{}{}", file_path, self.config.config_toml_suffix);
        let file = File::with_name(&config_path).required(false);
        config.merge(file)?;
        merger.merge_from(config);

        let config_path = format!("{}{}", file_path, self.config.config_json_suffix);
        let file = File::with_name(&config_path).required(false);
        config.merge(file)?;
        merger.merge_from(config);

        Ok(())
    }

    /// Creates a Config database by merging configuration files. The default toml and then json are merged. Then
    /// the environment variant toml and json are merged. Then, if a server flavor has been specified that flavor's
    /// folder is merged. First the default toml and json and then the environment variant toml and json files.
    /// Lastly, environment settings are merged.
    ///
    /// The enviroment variant represents where the server is being tested, geenrally, this is a Development, Test,
    /// Stage, or Production environment. However, it can be changed by the user.
    ///
    /// The server flavor provides a means of having a universal server with different personalities. Essentially,
    /// you have a server capable of many different functions, but only active for a few -- allowing the same
    /// image to be deployed everywhere, but fufilling different roles. For example, a single server might support
    /// any number of micro-services, however only a few may be desirable for each
    /// particular instance, by introducing flavors, you only need to set the enviroment properly or add it to the
    /// default when installing the server in the environment rather than having to edit many fields at once time.   
    fn create(&self, merger: &mut dyn ConfigMerger) -> Result<Config, ConfigError> {
        // Determine where we are located, default it if unknown
        let config_env_default = self.config.config_env_default.clone();
        let env = std::env::var(self.config.config_env_var.clone()).unwrap_or_else(|_| config_env_default.into());

        // Start with the default struct
        let mut s = Config::default();
        s.set("env", env.clone())?;

        // merge in the default toml and json config files
        let config_filepath = format!("{}{}", self.config.config_folder_path, self.config.config_default_name);
        self.merge_filepath(&mut s, &config_filepath, merger)?;

        // Merge in the ENV dependent toml and json config files
        let config_filepath = format!("{}{}", self.config.config_folder_path, env);
        self.merge_filepath(&mut s, &config_filepath, merger)?;

        // Try to deterime the server's flavor and load its config
        let var = self.config.config_env_var_server_flavor.clone();
        let default = self.config.config_env_var_server_flavor_default.clone();
        let default = s.get(&var.to_lowercase()).unwrap_or_else(|_| default);
        let server_flavor = std::env::var(var).unwrap_or_else(|_| default);

        if !server_flavor.is_empty() {
            // merge in the default toml and json config files
            let config_filepath = format!(
                "{}{}/{}",
                self.config.config_folder_path, server_flavor, self.config.config_default_name
            );
            self.merge_filepath(&mut s, &config_filepath, merger)?;

            // Merge in the ENV dependent toml and json config files
            let config_filepath = format!("{}{}/{}", self.config.config_folder_path, server_flavor, env);
            self.merge_filepath(&mut s, &config_filepath, merger)?;
        }
        merger.merge_into(&mut s)?;

        // Merge in environment overrides
        s.merge(Environment::with_prefix(&self.config.config_env_prefix).separator(&self.config.config_env_separator))?;
        Ok(s)
    }
}

/// The ServerConfig consists of two parts. The first part is meta-config that is
/// used to describe where to look for config files and how to parse environment
/// overrides. The second part is configuration information for the server.
#[derive(Debug, Deserialize, Clone)]
pub struct ConfigMetaData {
    pub config_env_var: String,
    pub config_env_default: String,
    pub config_env_var_server_flavor: String,
    pub config_env_var_server_flavor_default: String,
    pub config_env_prefix: String,
    pub config_env_separator: String,
    pub config_folder_path: String,
    pub config_toml_suffix: String,
    pub config_json_suffix: String,
    pub config_default_name: String,
}

impl Default for ConfigMetaData {
    fn default() -> Self {
        Self {
            config_env_var: CONFIG_ENV_VAR.to_string(),
            config_env_default: CONFIG_ENV_DEFAULT.to_string(),
            config_env_var_server_flavor: CONFIG_ENV_VAR_SERVER_FLAVOR.to_string(),
            config_env_var_server_flavor_default: CONFIG_ENV_VAR_SERVER_FLAVOR_DEFAULT.to_string(),
            config_env_prefix: CONFIG_ENV_PREFIX.to_string(),
            config_env_separator: CONFIG_ENV_SEPARATOR.to_string(),
            config_folder_path: CONFIG_FOLDER_PATH.to_string(),
            config_toml_suffix: CONFIG_TOML_SUFFIX.to_string(),
            config_json_suffix: CONFIG_JSON_SUFFIX.to_string(),
            config_default_name: CONFIG_DEFAULT_NAME.to_string(),
        }
    }
}
