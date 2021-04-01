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
    /// * `product_version` - the current product / controller version
    /// * `config_kind` - config kind provided by the user -> relate to config_option.option_name.kind
    /// * `config_file` - config file provided by the user -> relate to config_option.option_name.config_file
    /// * `config_role` - config role provided by the user -> relate to config_option.roles
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
    ///     &OptionKind::Env,
    ///     "env.sh",
    ///     Some("role_1"),
    ///     &user_data,
    /// );
    /// ```
    ///
    pub fn get(
        &self,
        product_version: &str,
        config_kind: &OptionKind,
        config_file: &str,
        config_role: Option<&str>,
        user_config: &HashMap<String, Option<String>>,
    ) -> HashMap<String, ProductConfigResult> {
        let mut result_config = HashMap::new();

        // collect all available options (user and config)
        let merged_config_options =
            self.merge_config_options(user_config, product_version, config_file, config_role);

        for (name, value) in &merged_config_options {
            //let mut result;
            let option_name = &OptionName {
                name: name.clone(),
                kind: config_kind.clone(),
                config_file: config_file.to_string(),
            };

            result_config.insert(
                option_name.name.clone(),
                validation::validate(
                    &self.config_options,
                    &self.config_setting_units,
                    &merged_config_options,
                    product_version,
                    config_role,
                    option_name,
                    value.clone(),
                ),
            );
        }
        result_config
    }

    /// Merge user config options and available config options depending on file and role to
    /// be validated later.
    ///
    /// # Arguments
    ///
    /// * `user_config` - map with option name and values (the explicit user config options)
    /// * `product_version` - the current product / controller version
    /// * `config_file` - config file provided by the user -> relate to config_option.option_name.config_file
    /// * `config_role` - config role provided by the user -> relate to config_option.roles
    ///
    fn merge_config_options(
        &self,
        user_config: &HashMap<String, Option<String>>,
        product_version: &str,
        config_file: &str,
        config_role: Option<&str>,
    ) -> HashMap<String, Option<String>> {
        let mut merged_config_options = HashMap::new();

        if let Ok(options) = util::filter_config_options(
            &self.config_options,
            config_file,
            config_role,
            product_version,
        ) {
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
            &OptionKind::Env,
            //"env.sh",
            "env.sh",
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
                    kind: OptionKind::Conf,
                    config_file: "my_config".to_string(),
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

    //     macro_rules! hashmap {
    //         ($( $key: expr => $val: expr ),*) => {{
    //              let mut map = ::std::collections::HashMap::new();
    //              $( map.insert($key, $val); )*
    //              map
    //         }}
    //     }
    //
    //     use crate::reader::ConfigJsonReader;
    //     use crate::{Config, Error, OptionKind, OptionName};
    //     use rstest::*;
    //     use std::collections::HashMap;
    //
    //     const V_1_0_0: &'static str = "1.0.0";
    //     const V_0_5_0: &'static str = "0.5.0";
    //     const V_0_1_0: &'static str = "0.1.0";
    //     const ENV_VAR_INTEGER_PORT_MIN_MAX: &'static str = "ENV_VAR_INTEGER_PORT_MIN_MAX";
    //     const CONF_PROPERTY_STRING_MEMORY: &'static str = "conf.property.string.memory";
    //     const CONF_PROPERTY_STRING_DEPRECATED: &'static str = "conf.property.string.deprecated";
    //     const ENV_VAR_ALLOWED_VALUES: &'static str = "ENV_VAR_ALLOWED_VALUES";
    //
    //     #[rstest(
    //         product_version,
    //         option,
    //         option_value,
    //         expected,
    //         case(
    //             V_1_0_0,
    //             OptionName { name: ENV_VAR_INTEGER_PORT_MIN_MAX.to_string(), kind: OptionKind::Env },
    //             Some("1000"),
    //             Ok(String::from("1000"))
    //         ),
    //         // test data type
    //         case(
    //             V_1_0_0,
    //             OptionName { name: ENV_VAR_INTEGER_PORT_MIN_MAX.to_string(), kind: OptionKind::Env },
    //             Some("abc"),
    //             Err(Error::DatatypeNotMatching{ option_name: OptionName { name: ENV_VAR_INTEGER_PORT_MIN_MAX.to_string(), kind: OptionKind::Env }, value: "abc".to_string(), datatype: "i64".to_string() })
    //         ),
    //         // test min bound
    //         case(
    //             V_1_0_0,
    //             OptionName { name: ENV_VAR_INTEGER_PORT_MIN_MAX.to_string(), kind: OptionKind::Env },
    //             Some("-1"),
    //             Err(Error::ConfigValueOutOfBounds{ option_name: OptionName { name: ENV_VAR_INTEGER_PORT_MIN_MAX.to_string(), kind: OptionKind::Env }, received: "-1".to_string(), expected: "0".to_string() })
    //         ),
    //         // test max bound
    //         case(
    //             V_1_0_0,
    //             OptionName { name: ENV_VAR_INTEGER_PORT_MIN_MAX.to_string(), kind: OptionKind::Env },
    //             Some("100000"),
    //             Err(Error::ConfigValueOutOfBounds{ option_name: OptionName { name: ENV_VAR_INTEGER_PORT_MIN_MAX.to_string(), kind: OptionKind::Env }, received: "100000".to_string(), expected: "65535".to_string() })
    //         ),
    //         // check version not supported
    //         case(
    //             V_0_1_0,
    //             OptionName { name: ENV_VAR_INTEGER_PORT_MIN_MAX.to_string(), kind: OptionKind::Env },
    //             Some("1000"),
    //             Err(Error::VersionNotSupported{ option_name: OptionName { name: ENV_VAR_INTEGER_PORT_MIN_MAX.to_string(), kind: OptionKind::Env }, product_version: V_0_1_0.to_string(), required_version: V_0_5_0.to_string() })
    //         ),
    //         case(
    //             V_0_5_0,
    //             OptionName { name: ENV_VAR_INTEGER_PORT_MIN_MAX.to_string(), kind: OptionKind::Env },
    //             Some("1000"),
    //             Ok(String::from("1000"))
    //         ),
    //         // check regex
    //         case(
    //             V_1_0_0,
    //             OptionName { name: CONF_PROPERTY_STRING_MEMORY.to_string(), kind: OptionKind::Conf },
    //             Some("abc"),
    //             Err(Error::DatatypeRegexNotMatching{ option_name: OptionName { name: CONF_PROPERTY_STRING_MEMORY.to_string(), kind: OptionKind::Conf }, value: "abc".to_string() })
    //         ),
    //         // check close regex
    //         case(
    //             V_1_0_0,
    //             OptionName { name: CONF_PROPERTY_STRING_MEMORY.to_string(), kind: OptionKind::Conf },
    //             Some("100"),
    //             Err(Error::DatatypeRegexNotMatching{ option_name: OptionName { name: CONF_PROPERTY_STRING_MEMORY.to_string(), kind: OptionKind::Conf }, value: "100".to_string() })
    //         ),
    //         case(
    //             V_1_0_0,
    //             OptionName { name: CONF_PROPERTY_STRING_MEMORY.to_string(), kind: OptionKind::Conf },
    //             Some("1000m"),
    //             Ok(String::from("1000m"))
    //         ),
    //         case(
    //             V_1_0_0,
    //             OptionName { name: CONF_PROPERTY_STRING_MEMORY.to_string(), kind: OptionKind::Conf },
    //             Some("100mb"),
    //             Ok(String::from("100mb"))
    //         ),
    //         // check deprecated
    //         case(
    //             V_0_5_0,
    //             OptionName { name: CONF_PROPERTY_STRING_DEPRECATED.to_string(), kind: OptionKind::Conf },
    //             Some("1000m"),
    //             Err(Error::VersionDeprecated { option_name: OptionName { name: CONF_PROPERTY_STRING_DEPRECATED.to_string(), kind: OptionKind::Conf }, product_version: V_0_5_0.to_string(), deprecated_version: "0.4.0".to_string() })
    //         ),
    //         // check allowed values
    //         case(
    //             V_0_5_0,
    //             OptionName { name: ENV_VAR_ALLOWED_VALUES.to_string(), kind: OptionKind::Env },
    //             Some("allowed_value1"),
    //             Ok(String::from("allowed_value1"))
    //         ),
    //         case(
    //             V_0_5_0,
    //             OptionName { name: ENV_VAR_ALLOWED_VALUES.to_string(), kind: OptionKind::Env },
    //             Some("abc"),
    //             Err(Error::ConfigValueNotInAllowedValues{ option_name: OptionName { name: ENV_VAR_ALLOWED_VALUES.to_string(), kind: OptionKind::Env }, value: "abc".to_string(), allowed_values: vec![String::from("allowed_value1"), String::from("allowed_value2"), String::from("allowed_value3")] })
    //         ),
    //         // empty string?
    //         case(
    //             V_0_5_0,
    //             OptionName { name: ENV_VAR_ALLOWED_VALUES.to_string(), kind: OptionKind::Env },
    //             Some(""),
    //             Ok(String::from(""))
    //         ),
    //         // None
    //         case(
    //             V_0_5_0,
    //             OptionName { name: ENV_VAR_ALLOWED_VALUES.to_string(), kind: OptionKind::Env },
    //             None,
    //             Err(Error::ConfigValueMissing { option_name: OptionName { name: ENV_VAR_ALLOWED_VALUES.to_string(), kind: OptionKind::Env } })
    //         ),
    //         ::trace
    //     )]
    //     fn test_validate(
    //         product_version: &str,
    //         option: OptionName,
    //         option_value: Option<&str>,
    //         expected: Result<String, Error>,
    //     ) {
    //         let reader = ConfigJsonReader::new("data/test_config.json");
    //         let config = Config::new(reader).unwrap();
    //         let result = config.validate(product_version, &option.kind, &option.name, option_value, "");
    //         assert_eq!(result, expected)
    //     }
    //
    //     const ENV_SSL_CERTIFICATE_PATH: &'static str = "ENV_SSL_CERTIFICATE_PATH";
    //     const ENV_SSL_ENABLED: &'static str = "ENV_SSL_ENABLED";
    //     const PATH_TO_CERTIFICATE: &str = "some/path/to/certificate";
    //
    //     #[rstest(
    //         product_version,
    //         options,
    //         expected,
    //         // missing dependency
    //         case(
    //             "1.0.0",
    //             hashmap!{
    //                 OptionName { name: ENV_SSL_CERTIFICATE_PATH.to_string(), kind: OptionKind::Env } => Some("some/path/to/certificate".to_string())
    //             },
    //             Err(Error::ConfigDependencyMissing { option_name: OptionName { name: ENV_SSL_CERTIFICATE_PATH.to_string(), kind: OptionKind::Env }, dependency: "ENV_SSL_ENABLED".to_string() })
    //         ),
    //         // correct dependency
    //         case(
    //             "1.0.0",
    //             hashmap!{
    //                 OptionName { name: ENV_SSL_CERTIFICATE_PATH.to_string(), kind: OptionKind::Env } => Some(PATH_TO_CERTIFICATE.to_string()),
    //                 OptionName { name: ENV_SSL_ENABLED.to_string(), kind: OptionKind::Env } => Some("true".to_string())
    //             },
    //             Ok(())
    //         ),
    //         // correct dependency, wrong value
    //         case(
    //             "1.0.0",
    //             hashmap!{
    //                 OptionName { name: ENV_SSL_CERTIFICATE_PATH.to_string(), kind: OptionKind::Env } => Some(PATH_TO_CERTIFICATE.to_string()),
    //                 OptionName { name: ENV_SSL_ENABLED.to_string(), kind: OptionKind::Env } => Some("false".to_string())
    //             },
    //             Err(Error::ConfigDependencyValueInvalid { option_name: OptionName { name: ENV_SSL_CERTIFICATE_PATH.to_string(), kind: OptionKind::Env }, dependency: ENV_SSL_ENABLED.to_string(), option_value: "false".to_string(), required_value: "true".to_string() })
    //         ),
    //         ::trace
    //     )]
    //     fn test_validate_all(
    //         product_version: &str,
    //         options: HashMap<OptionName, Option<String>>,
    //         expected: Result<(), Error>,
    //     ) {
    //         let reader = ConfigJsonReader::new("data/test_config.json");
    //         let config = Config::new(reader).unwrap();
    //         let result = config.validate_all(product_version, &options);
    //         assert_eq!(result, expected)
    //     }
}
