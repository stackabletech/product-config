//! A library to provide generalized access to specified product configuration options
//!
//! Validation of configuration options and values in terms of:
//! - matching data types (e.g. integer, bool, string...)
//! - minimal and maximal possible values
//! - regex expressions for different units like port, url, ip etc.
//! - version and deprecated checks
//! - support for default and recommended values depending on version
//! - dependency checks for values that require other values to be set to a certain value
//! - options can be assigned to certain rules (server, client ...)
//! - apply mode for config changes (e.g. restart)
//! - additional information like web links or descriptions
//!
//! For now, the product config is build from e.g. a JSON file like "../data/test_config.json":
//! The JSON example is defined as ConfigItem and is split into config_settings and config_options
//!  - config_settings contains additional information (e.g. like unit and respective regex patterns)
//!  - config_options contains all the possible configuration options including all the know how for validation
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
use crate::types::{ConfigKind, ConfigName, ConfigOption, ConfigSpec};
use crate::validation::ValidationResult;
use regex::Regex;
use semver::Version;

/// This will be returned for every validated configuration value (including user values
/// and automatically added values from e.g. dependency, recommended etc.).
#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub enum ConfigOptionValidationResult {
    /// On Default, the provided value does not differ from the default settings and may be
    /// left out from the user config in the future.
    Default(String),
    /// On RecommendedDefault, the value for this configuration option is a recommended value.
    /// Will be returned when the user did not provide a value and the product does not have a default.
    RecommendedDefault(String),
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
    config_options: HashMap<ConfigName, ConfigOption>,
}

impl ProductConfig {
    /// Create a ProductConfig based on a config reader like e.g. JSON, YAML etc.
    ///
    /// # Arguments
    ///
    /// * `config_reader` - config_reader implementation
    ///
    pub fn new<CR: ConfigReader<ConfigSpec>>(config_reader: CR) -> ValidationResult<Self> {
        let config = config_reader.read()?;
        let product_config = parse_config_spec(&config)?;

        validation::validate_config_options(
            &product_config.config_options,
            &product_config.config_setting_units,
        )?;

        Ok(product_config)
    }

    /// Retrieve and check config options depending on the config option kind (e.g. env, conf),
    /// the required config file (e.g. environment variables or config properties).
    /// Add other provided options that match the config kind, config file and config role.
    /// Automatically add and correct missing or wrong config options and dependencies.
    ///
    /// # Arguments
    ///
    /// * `version` - the current product version
    /// * `kind` - config kind provided by the user -> relate to config_option.option_name.kind
    /// * `role` - config role provided by the user -> relate to config_option.roles
    /// * `user_config` - map with option name and values (the explicit user config options)
    ///
    /// # Examples
    ///
    /// ```
    /// use product_config::reader::ConfigJsonReader;
    /// use product_config::types::ConfigKind;
    /// use product_config::ProductConfig;
    /// use std::collections::HashMap;
    ///
    /// let config = ProductConfig::new(ConfigJsonReader::new("data/test_config.json")).unwrap();
    ///
    /// let mut user_data = HashMap::new();
    /// user_data.insert("ENV_INTEGER_PORT_MIN_MAX".to_string(), "12345".to_string());
    /// user_data.insert("ENV_PROPERTY_STRING_MEMORY".to_string(), "1g".to_string());
    ///
    /// let env_sh = config.get(
    ///     "0.5.0",
    ///     &ConfigKind::Conf("env.sh".to_string()),
    ///     Some("role_1"),
    ///     &user_data,
    /// );
    /// ```
    ///
    pub fn get(
        &self,
        version: &str,
        kind: &ConfigKind,
        role: Option<&str>,
        user_config: &HashMap<String, String>,
    ) -> ValidationResult<HashMap<String, ConfigOptionValidationResult>> {
        let mut result_config = HashMap::new();

        let product_version = Version::parse(version)?;

        // merge provided user options with extracted config options via role / kind and
        // dependencies to be validated later.
        let merged_config_options =
            self.merge_config_options(user_config, &product_version, kind, role);

        for (name, option_value) in &merged_config_options {
            let option_name = &ConfigName {
                name: name.clone(),
                kind: kind.clone(),
            };

            result_config.insert(
                option_name.name.clone(),
                validation::validate(
                    &self.config_options,
                    &self.config_setting_units,
                    &merged_config_options,
                    &product_version,
                    role,
                    option_name,
                    option_value,
                ),
            );
        }

        Ok(result_config)
    }

    /// Merge provided user config options and available config options (from JSON, YAML...)
    /// depending on kind and role to be validated later.
    ///
    /// # Arguments
    ///
    /// * `user_config` - map with option name and values (the explicit user config options)
    /// * `version` - the current product version
    /// * `kind` - config kind provided by the user -> relate to config_option.option_name.kind
    /// * `role` - config role provided by the user -> relate to config_option.roles
    ///
    fn merge_config_options(
        &self,
        user_config: &HashMap<String, String>,
        version: &Version,
        kind: &ConfigKind,
        role: Option<&str>,
    ) -> HashMap<String, String> {
        let mut merged_config_options = HashMap::new();

        if let Ok(options) =
            util::get_matching_config_options(&self.config_options, kind, role, version)
        {
            merged_config_options.extend(options)
        }

        if let Ok(dependencies) =
            util::get_matching_dependencies(&self.config_options, user_config, version, kind)
        {
            merged_config_options.extend(dependencies);
        }

        merged_config_options.extend(user_config.clone());

        merged_config_options
    }
}

/// Parse the provided config spec. Store config options in a hashmap with the option name
/// as key. Parse any additional settings like units and the respective regex patterns.
///
/// # Arguments
///
/// * `config_spec` - the config spec provided by the ConfigReader (JSON, ...)
///
fn parse_config_spec(config_spec: &ConfigSpec) -> ValidationResult<ProductConfig> {
    let mut config_options = HashMap::new();
    // pack config item options via name into hashmap for access
    for config_option in config_spec.config_options.iter() {
        // for every provided config option name, write config option reference into map
        for option_name in config_option.config_names.iter() {
            config_options.insert(option_name.clone(), config_option.clone());
        }
    }

    let mut config_setting_units = HashMap::new();
    // pack unit name and compiled regex pattern into map
    for unit in &config_spec.config_settings.units {
        let config_setting_unit_name = if unit.name.is_empty() {
            return Err(Error::ConfigSettingNotFound {
                name: "unit".to_string(),
            });
        } else {
            unit.name.clone()
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
    use crate::types::{ConfigKind, ConfigName};
    use crate::{ConfigOptionValidationResult, ProductConfig};
    use rstest::*;
    use std::collections::HashMap;

    const ENV_INTEGER_PORT_MIN_MAX: &str = "ENV_INTEGER_PORT_MIN_MAX";

    const ENV_FLOAT: &str = "ENV_FLOAT";
    //const ENV_PROPERTY_STRING_MEMORY: &str = "ENV_PROPERTY_STRING_MEMORY";
    //const ENV_PROPERTY_STRING_DEPRECATED: &str = "ENV_PROPERTY_STRING_DEPRECATED";
    //const ENV_ALLOWED_VALUES: &str = "ENV_ALLOWED_VALUES";
    //const ENV_SECURITY: &str = "ENV_SECURITY";
    //const ENV_SECURITY_PASSWORD: &str = "ENV_SECURITY_PASSWORD";
    const ENV_SSL_ENABLED: &str = "ENV_SSL_ENABLED";
    const ENV_SSL_CERTIFICATE_PATH: &str = "ENV_SSL_CERTIFICATE_PATH";

    const ROLE_1: &str = "role_1";
    const VERSION_0_5_0: &str = "0.5.0";
    const CONF_FILE: &str = "env.sh";

    fn create_empty_data_and_expected() -> (
        HashMap<String, String>,
        HashMap<String, ConfigOptionValidationResult>,
    ) {
        let float_recommended = "50.0";
        let port_recommended = "20000";

        let data = HashMap::new();

        let mut expected = HashMap::new();
        expected.insert(
            ENV_INTEGER_PORT_MIN_MAX.to_string(),
            ConfigOptionValidationResult::RecommendedDefault(port_recommended.to_string()),
        );
        expected.insert(
            ENV_FLOAT.to_string(),
            ConfigOptionValidationResult::RecommendedDefault(float_recommended.to_string()),
        );
        (data, expected)
    }

    fn create_correct_data_and_expected() -> (
        HashMap<String, String>,
        HashMap<String, ConfigOptionValidationResult>,
    ) {
        let port = "12345";
        let ssl_enabled = "true";
        let certificate_path = "/tmp/ssl_key.xyz";
        let float_value = "55.555";

        let mut data = HashMap::new();
        data.insert(ENV_INTEGER_PORT_MIN_MAX.to_string(), port.to_string());
        data.insert(
            ENV_SSL_CERTIFICATE_PATH.to_string(),
            certificate_path.to_string(),
        );
        data.insert(ENV_FLOAT.to_string(), float_value.to_string());

        let mut expected = HashMap::new();

        expected.insert(
            ENV_INTEGER_PORT_MIN_MAX.to_string(),
            ConfigOptionValidationResult::Valid(port.to_string()),
        );
        expected.insert(
            ENV_SSL_CERTIFICATE_PATH.to_string(),
            ConfigOptionValidationResult::Valid(certificate_path.to_string()),
        );
        expected.insert(
            ENV_SSL_ENABLED.to_string(),
            ConfigOptionValidationResult::RecommendedDefault(ssl_enabled.to_string()),
        );
        expected.insert(
            ENV_FLOAT.to_string(),
            ConfigOptionValidationResult::Valid(float_value.to_string()),
        );

        (data, expected)
    }

    #[rstest(
        version,
        kind,
        role,
        user_data,
        expected,
        case(
            VERSION_0_5_0,
            &ConfigKind::Conf(CONF_FILE.to_string()),
            Some(ROLE_1),
            create_empty_data_and_expected().0,
            create_empty_data_and_expected().1,
        ),
        case(
            VERSION_0_5_0,
            &ConfigKind::Conf(CONF_FILE.to_string()),
            Some(ROLE_1),
            create_correct_data_and_expected().0,
            create_correct_data_and_expected().1,
        ),
    ::trace
    )]
    fn test_get_kind_conf_role_1(
        version: &str,
        kind: &ConfigKind,
        role: Option<&str>,
        user_data: HashMap<String, String>,
        expected: HashMap<String, ConfigOptionValidationResult>,
    ) {
        let config = ProductConfig::new(ConfigJsonReader::new("data/test_config.json")).unwrap();

        let result = config.get(version, kind, role, &user_data).unwrap();

        println!("Size: {}", result.len());
        for x in &result {
            println!("{:?}", x)
        }

        assert_eq!(result, expected);
    }

    #[test]
    fn test_product_config_result_order() {
        let valid = ConfigOptionValidationResult::Valid("valid".to_string());
        let default = ConfigOptionValidationResult::Default("default".to_string());
        let recommended =
            ConfigOptionValidationResult::RecommendedDefault("recommended".to_string());
        let warn = ConfigOptionValidationResult::Warn(
            "warning".to_string(),
            Error::ConfigOptionNotFound {
                option_name: ConfigName {
                    name: "test".to_string(),
                    kind: ConfigKind::Conf("my_config".to_string()),
                },
            },
        );
        let error = ConfigOptionValidationResult::Error(Error::ConfigSettingNotFound {
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
