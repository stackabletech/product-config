//! A library to provide generalized access to specified product configuration options
//!
//! Validation of configuration options and values in terms of:
//! - matching data types (e.g. integer, bool, string...)
//! - minimal and maximal possible values
//! - regex expressions for different units like port, url, ip etc.
//! - version and deprecated checks
//! - support for default and recommended values depending on version
//! - dependency checks for values that require other values to be set to a certain value
//!   
//! Additional information like web links or descriptions
//!
//! For now, the product config is build from e.g. a JSON file like "../data/test_config.json":
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
use crate::validation::ConfigValidationResult;
use regex::Regex;

#[derive(Clone, Debug, PartialOrd, PartialEq)]
#[repr(u8)]
pub enum ProductConfigResult {
    /// On Default, the value does not differ from the default settings and may be
    /// left out from the user config in the future.
    Default(String),
    /// On Recommended, the value from the recommended section depending
    /// on the product version was used. May be because of automatic enhancement,
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
    pub fn new<CR: ConfigReader<ConfigItem>>(config_reader: CR) -> ConfigValidationResult<Self> {
        let config = config_reader.read()?;
        let product_config = parse_config(&config);
        match &product_config {
            Ok(conf) => validation::validate_config_options(
                &conf.config_options,
                &conf.config_setting_units,
            )?,
            Err(err) => return Err(err.clone()),
        }

        product_config
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
    /// user_data.insert("ENV_INTEGER_PORT_MIN_MAX".to_string(), "12345".to_string());
    /// user_data.insert("ENV_PROPERTY_STRING_MEMORY".to_string(), "1g".to_string());
    ///
    /// let env_sh = config.get(
    ///     "0.5.0",
    ///     &OptionKind::Conf("env.sh".to_string()),
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
        user_config: &HashMap<String, String>,
    ) -> HashMap<String, ProductConfigResult> {
        let mut result_config = HashMap::new();

        // collect all available options (user and config)
        let merged_config_options = self.merge_config_options(user_config, version, kind, role);

        for (name, option_value) in &merged_config_options {
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
                    option_value,
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
        user_config: &HashMap<String, String>,
        version: &str,
        kind: &OptionKind,
        role: Option<&str>,
    ) -> HashMap<String, String> {
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
fn parse_config(config: &ConfigItem) -> ConfigValidationResult<ProductConfig> {
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
        HashMap<String, ProductConfigResult>,
    ) {
        let ssl_enabled = "true";
        let float_recommended = "50.0";
        let port_recommended = "20000";

        let data = HashMap::new();

        let mut expected = HashMap::new();
        expected.insert(
            ENV_INTEGER_PORT_MIN_MAX.to_string(),
            ProductConfigResult::Recommended(port_recommended.to_string()),
        );
        expected.insert(
            ENV_SSL_ENABLED.to_string(),
            ProductConfigResult::Recommended(ssl_enabled.to_string()),
        );
        expected.insert(
            ENV_FLOAT.to_string(),
            ProductConfigResult::Recommended(float_recommended.to_string()),
        );
        (data, expected)
    }

    fn create_correct_data_and_expected() -> (
        HashMap<String, String>,
        HashMap<String, ProductConfigResult>,
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
        data.insert(ENV_SSL_ENABLED.to_string(), ssl_enabled.to_string());
        data.insert(ENV_FLOAT.to_string(), float_value.to_string());

        let mut expected = HashMap::new();

        expected.insert(
            ENV_INTEGER_PORT_MIN_MAX.to_string(),
            ProductConfigResult::Valid(port.to_string()),
        );
        expected.insert(
            ENV_SSL_CERTIFICATE_PATH.to_string(),
            ProductConfigResult::Valid(certificate_path.to_string()),
        );
        expected.insert(
            ENV_SSL_ENABLED.to_string(),
            ProductConfigResult::Recommended(ssl_enabled.to_string()),
        );
        expected.insert(
            ENV_FLOAT.to_string(),
            ProductConfigResult::Valid(float_value.to_string()),
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
            &OptionKind::Conf(CONF_FILE.to_string()),
            Some(ROLE_1),
            create_empty_data_and_expected().0,
            create_empty_data_and_expected().1,
        ),
        case(
            VERSION_0_5_0,
            &OptionKind::Conf(CONF_FILE.to_string()),
            Some(ROLE_1),
            create_correct_data_and_expected().0,
            create_correct_data_and_expected().1,
        ),
    ::trace
    )]
    fn test_get_kind_conf_role_1(
        version: &str,
        kind: &OptionKind,
        role: Option<&str>,
        user_data: HashMap<String, String>,
        expected: HashMap<String, ProductConfigResult>,
    ) {
        let config = ProductConfig::new(ConfigJsonReader::new("data/test_config.json")).unwrap();

        let result = config.get(version, kind, role, &user_data);

        println!("Size: {}", result.len());
        for x in &result {
            println!("{:?}", x)
        }

        assert_eq!(result, expected);
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
