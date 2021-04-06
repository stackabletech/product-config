//! A library to provide generalized access to specified product configuration options
//!
//! Validation of configuration options and values in terms of:
//! - matching data types (e.g. integer, bool, string...)
//! - minimal and maximal possible values
//! - regex expressions for different units
//! - version and deprecated checks
//! - support for default values depending on version
//!   
//! Additional information like web links or descriptions
//!
//! The product config is build from e.g. a JSON file like in the example below:
//! - The whole example is defined as ConfigItem and is split into config_settings and config_options
//!   * config_settings contains additional information (e.g. like unit and respective regex patterns)
//!   * config_options contains all the possible configuration options including all the know how for validation
//!
pub mod error;
pub mod reader;
pub mod types;
mod util;
mod validation;

use std::collections::HashMap;
use std::str;
use std::string::String;

use crate::error::Error;
use crate::reader::ConfigReader;
use crate::types::{ConfigItem, ConfigOption, OptionKind, OptionName};
use regex::Regex;

#[derive(Clone, Debug, PartialOrd, PartialEq)]
#[repr(u8)]
pub enum ProductConfigResult {
    /// On Default, the value does not differ from the default settings and may be
    /// left out from the user config in the future.
    Default(String),
    /// On Recommended, the value from the recommended section depending
    /// on the product version was used. Can be because of automatic enhancement,
    /// matching config file and role etc.
    Recommended(String),
    /// On Valid, the value passed all checks and can be used.
    Valid(String),
    /// On warn, the value maybe used with caution.
    Warn(String, Error),
    /// On error, check the provided config and config values.
    /// Should never be used like this!
    Error(Error),
}

#[derive(Debug)]
pub struct ProductConfig {
    // provided config units with corresponding regex pattern
    config_setting_units: HashMap<String, Regex>,
    // option names as key and the corresponding option as value
    config_options: HashMap<OptionName, ConfigOption>,
}

impl ProductConfig {
    /// Create a ProductConfig based on a config reader like e.g. JSON, YAML etc.
    ///
    /// # Arguments
    ///
    /// * `config_reader` - config_reader implementation
    ///
    pub fn new<CR: ConfigReader<ConfigItem>>(config_reader: CR) -> Result<Self, Error> {
        let config = config_reader.read()?;
        parse_config(&config)
    }

    /// Retrieve and check config options depending on the kind (e.g. env, conf), the required config file
    /// (e.g. environment variables or config properties). Add other provided options that match the
    /// config kind, config file and config role. Automatically add and correct missing or wrong
    /// config options and dependencies.
    ///
    /// # Arguments
    ///
    /// * `version` - the current product / controller version
    /// * `kind` - config kind provided by the user -> relate to config_option.option_name.kind
    /// * `role` - config role provided by the user -> relate to config_option.roles
    /// * `user_config` - map with option name and values (the explicit user config options)
    ///
    /// # Examples
    ///
    /// ```
    /// use product_config::reader::ConfigJsonReader;
    /// use product_config::types::OptionKind;
    /// use product_config::ProductConfig;
    /// use std::collections::HashMap;
    ///
    /// let config = ProductConfig::new(ConfigJsonReader::new("data/test_config.json")).unwrap();
    ///
    /// let mut user_data = HashMap::new();
    /// user_data.insert("ENV_VAR_INTEGER_PORT_MIN_MAX".to_string(), Some("12345".to_string()));
    /// user_data.insert("ENV_PROPERTY_STRING_MEMORY".to_string(), Some("1g".to_string()));
    ///
    /// let env_sh = config.get(
    ///     "0.5.0",
    ///     &OptionKind::Env("env.sh".to_string()),
    ///     Some("role_1"),
    ///     &user_data,
    /// );
    /// ```
    ///
    pub fn get(
        &self,
        version: &str,
        kind: &OptionKind,
        role: Option<&str>,
        user_config: &HashMap<String, Option<String>>,
    ) -> HashMap<String, ProductConfigResult> {
        let mut result_config = HashMap::new();

        // collect all available options (user and config)
        let merged_config_options = self.merge_config_options(user_config, version, kind, role);

        for (name, value) in &merged_config_options {
            let option_name = &OptionName {
                name: name.clone(),
                kind: kind.clone(),
            };

            result_config.insert(
                option_name.name.clone(),
                validation::validate(
                    &self.config_options,
                    &self.config_setting_units,
                    &merged_config_options,
                    version,
                    role,
                    option_name,
                    value.clone(),
                ),
            );
        }
        result_config
    }

    /// Merge user config options and available config options depending on kind and role to
    /// be validated later.
    ///
    /// # Arguments
    ///
    /// * `user_config` - map with option name and values (the explicit user config options)
    /// * `version` - the current product / controller version
    /// * `kind` - config kind provided by the user -> relate to config_option.option_name.kind
    /// * `role` - config role provided by the user -> relate to config_option.roles
    ///
    fn merge_config_options(
        &self,
        user_config: &HashMap<String, Option<String>>,
        version: &str,
        kind: &OptionKind,
        role: Option<&str>,
    ) -> HashMap<String, Option<String>> {
        let mut merged_config_options = HashMap::new();

        if let Ok(options) = util::filter_config_options(&self.config_options, kind, role, version)
        {
            merged_config_options.extend(options)
        }

        merged_config_options.extend(user_config.clone());

        merged_config_options
    }
}

/// Retrieve and check config options depending on the kind (e.g. env, conf), the required config file
///
/// # Arguments
///
/// * `config` - the current product / controller version
///
fn parse_config(config: &ConfigItem) -> Result<ProductConfig, Error> {
    let mut config_options: HashMap<OptionName, ConfigOption> = HashMap::new();
    // pack config item options via name into hashmap for access
    for config_option in config.config_options.iter() {
        // for every provided config option name, write config option reference into map
        for option_name in config_option.option_names.iter() {
            config_options.insert(option_name.clone(), config_option.clone());
        }
    }

    let mut config_setting_units: HashMap<String, Regex> = HashMap::new();
    // pack unit name and compiled regex pattern into map
    for unit in &config.config_settings.unit {
        let config_setting_unit_name = if !unit.name.is_empty() {
            unit.name.clone()
        } else {
            return Err(Error::ConfigSettingNotFound {
                name: "unit".to_string(),
            });
        };

        // no regex or empty regex provided
        let config_setting_unit_regex =
            if unit.regex.is_none() || unit.regex == Some("".to_string()) {
                return Err(Error::EmptyRegexPattern {
                    unit: unit.name.clone(),
                });
            } else {
                unit.regex.clone().unwrap()
            };

        let regex = match Regex::new(config_setting_unit_regex.as_str()) {
            Ok(regex) => regex,
            Err(_) => {
                return Err(Error::InvalidRegexPattern {
                    unit: config_setting_unit_name,
                    regex: config_setting_unit_regex,
                });
            }
        };

        config_setting_units.insert(config_setting_unit_name, regex);
    }

    Ok(ProductConfig {
        config_setting_units,
        config_options,
    })
}

#[cfg(test)]
mod tests {
    use crate::error::Error;
    use crate::reader::ConfigJsonReader;
    use crate::types::{OptionKind, OptionName};
    use crate::{ProductConfig, ProductConfigResult};
    use std::collections::HashMap;

    #[test]
    fn test() {
        let config = ProductConfig::new(ConfigJsonReader::new("data/test_config.json")).unwrap();

        let mut test_data = HashMap::new();
        test_data.insert(
            "ENV_VAR_INTEGER_PORT_MIN_MAX".to_string(),
            Some("123456".to_string()),
        );
        test_data.insert(
            "ENV_PROPERTY_STRING_MEMORY".to_string(),
            Some("1g".to_string()),
        );

        test_data.insert(
            "ENV_SSL_CERTIFICATE_PATH".to_string(),
            Some("/tmp/ssl_key.xyz".to_string()),
        );

        let temp = config.get(
            "0.5.0",
            &OptionKind::Env("env.sh".to_string()),
            Some("role_1"),
            &test_data,
        );

        println!("Size: {}", temp.len());
        for x in temp {
            println!("{:?}", x)
        }
    }

    #[test]
    fn test_product_config_result_order() {
        let valid = ProductConfigResult::Valid("valid".to_string());
        let default = ProductConfigResult::Default("default".to_string());
        let recommended = ProductConfigResult::Recommended("recommended".to_string());
        let warn = ProductConfigResult::Warn(
            "warning".to_string(),
            Error::ConfigOptionNotFound {
                option_name: OptionName {
                    name: "test".to_string(),
                    kind: OptionKind::Conf("my_config".to_string()),
                },
            },
        );
        let error = ProductConfigResult::Error(Error::ConfigSettingNotFound {
            name: "xyz".to_string(),
        });

        assert!(valid > recommended);
        assert!(valid > default);
        assert!(valid < error);

        assert!(warn < error);
        assert!(error > warn);
        assert_eq!(error, error);
    }
}
