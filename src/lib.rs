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
//! # Example
//! {
//!    "config_settings":{
//!       "unit":[
//!          {
//!             "name":"memory",
//!             "regex":"(^\\p{N}+)(?:\\s*)((?:b|k|m|g|t|p|kb|mb|gb|tb|pb)\\b$)"
//!          },
//!          {
//!             "name":"number",
//!             "regex":"^-?[0-9][0-9,\\.]+$"
//!          }
//!       ]
//!    },
//!    "config_options":[
//!       {
//!          "option_names":[
//!             {
//!                "name":"HTTP_PORT",
//!                "kind":"http.port"
//!             },
//!             {
//!                "name":"http.port",
//!                "kind":"conf"
//!             }
//!          ],
//!          "datatype":{
//!             "type":"integer",
//!             "min":"0",
//!             "max":"65535",
//!             "unit":"port"
//!          },
//!          "as_of_version":"0.5.0",
//!          "deprecated_since":"1.0.0",
//!          "deprecated_for":[
//!             [
//!                {
//!                   "name":"NEW_HTTP_PORT",
//!                   "kind":"env"
//!                },
//!                {
//!                   "name":"new.http.port",
//!                   "kind":"conf"
//!                }
//!             ]
//!          ]
//!       },
//!       {
//!          "option_names":[
//!             {
//!                "name":"PRODUCT_MEMORY",
//!                "kind":"env"
//!             },
//!             {
//!                "name":"product.memory",
//!                "kind":"conf"
//!             },
//!             {
//!                "name":"mem",
//!                "kind":"cli"
//!             }
//!          ],
//!          "default_value":[
//!             {
//!                "from_version":"1.0.0",
//!                "value":"1g"
//!             }
//!          ],
//!          "datatype":{
//!             "type":"string",
//!             "unit":"memory"
//!          },
//!          "allowed_values":[
//!             "1g",
//!             "2g",
//!             "4g"
//!          ],
//!          "as_of_version":"1.0.0",
//!          "depends_on": [
//!            {
//!               "option_names":[
//!                  {
//!                     "name":"ANOTHER_PROPERTY",
//!                     "kind":"env"
//!                  }
//!               ],
//!               "value": true
//!             }
//!          ],
//!          "importance":"required",
//!          "apply_mode": "restart"
//!          "additional_doc":"http://additional.doc",
//!          "description":"Set the memory for x"
//!       }
//!    ]
//! }
//!
pub mod error;
pub mod reader;

use serde::Deserialize;
use std::collections::HashMap;
use std::str;
use std::str::FromStr;
use std::string::String;

use crate::error::Error;
use crate::reader::ConfigReader;
use regex::Regex;
use semver::Version;
use std::fmt::Display;

pub type Result<T> = std::result::Result<T, error::Error>;

pub enum Validation {
    Ok,
    Invalid { reasons: Vec<error::Error> },
}

#[derive(Debug)]
pub struct Config {
    // provided config units with corresponding regex pattern
    pub config_setting_units: HashMap<String, Regex>,
    // option names as key and the corresponding option as value
    pub config_options: HashMap<String, ConfigOption>,
}

impl Config {
    /// Returns a Config with data loaded from the config reader
    ///
    /// # Arguments
    ///
    /// * `config_reader` - A config reader implementing the trait ConfigReader
    ///
    /// # Examples
    ///
    /// ```
    /// ```
    pub fn new<CR: ConfigReader<ConfigItem>>(config_reader: CR) -> Result<Self> {
        let root = config_reader.read()?;

        let mut config_options_map: HashMap<String, ConfigOption> = HashMap::new();
        // pack config item options via name into hashmap for access
        for config_option in root.config_options.iter() {
            // for every provided config option name, write config option reference into map
            for option_name in config_option.option_names.iter() {
                config_options_map.insert(option_name.name.clone(), config_option.clone());
            }
        }

        let mut config_setting_units_map: HashMap<String, Regex> = HashMap::new();
        // pack unit name and compiled regex pattern into map
        for unit in &root.config_settings.unit {
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

            config_setting_units_map.insert(config_setting_unit_name, regex);
        }

        Ok(Config {
            config_setting_units: config_setting_units_map,
            config_options: config_options_map,
        })
    }

    /// Returns the provided option_value if no validation errors appear
    ///
    /// # Arguments
    ///
    /// * `product_version` - version of the currently active product version
    /// * `option_name` - name of the config option (config property or environmental variable)
    /// * `option_value` - config option value to be validated; Option.None means missing, Option<""> will avoid some checks and set option to empty
    ///
    /// # Examples
    ///
    /// ```
    /// ```
    pub fn validate(
        &self,
        product_version: &str,
        option_name: &str,
        option_value: Option<&str>,
    ) -> Result<String> {
        // a missing / wrong config option stops us from doing any other validation
        if !self.config_options.contains_key(option_name) {
            return Err(Error::ConfigOptionNotFound {
                option_name: option_name.to_string(),
            });
        }

        let option = self.config_options.get(option_name).unwrap();

        let value = match option_value {
            None => {
                // value missing is just an error
                return Err(Error::ConfigValueMissing {
                    option_name: option_name.to_string(),
                });
            }
            Some(val) => val,
        };

        // checks for config option
        self.check_version_supported_or_deprecated(
            option_name,
            product_version,
            &option.as_of_version[..],
            &option.deprecated_since,
        )?;

        // for an empty value (""), ignore checks for the value (check_datatype, check_allowed_values..)
        if !value.is_empty() {
            self.check_datatype(option_name, value, &option.datatype)?;
            self.check_allowed_values(option_name, value, &option.allowed_values)?;
        }

        Ok(value.to_string())
    }

    pub fn validate_all(
        &self,
        product_version: &str,
        options: &HashMap<String, String>,
    ) -> Result<()> {
        for (option_name, option_value) in options {
            // single option validation
            self.validate(
                product_version,
                option_name.as_str(),
                Some(option_value.as_str()),
            )?;
        }

        // additional dependency validation
        self.check_dependencies(&options)?;

        Ok(())
    }

    /// Check if config option version is supported or deprecated regarding the product version
    /// # Arguments
    ///
    /// * `option_name` - name of the config option (config property or environmental variable)
    /// * `product_version` - product / controller version
    /// * `option_version` - as of version of the provided config option
    /// * `deprecated_since` - version from which point onwards the option is deprecated
    ///
    fn check_version_supported_or_deprecated(
        &self,
        option_name: &str,
        product_version: &str,
        option_version: &str,
        deprecated_since: &Option<String>,
    ) -> Result<()> {
        let product_version = Version::parse(product_version)?;
        let option_version = Version::parse(option_version)?;

        // compare version of the config option and product / controller version
        if option_version > product_version {
            return Err(Error::VersionNotSupported {
                option_name: option_name.to_string(),
                product_version: product_version.to_string(),
                required_version: option_version.to_string(),
            });
        }

        // check if requested config option is deprecated
        if let Some(deprecated) = deprecated_since {
            let deprecated_since_version = Version::parse(deprecated.as_ref())?;

            if deprecated_since_version <= product_version {
                return Err(Error::VersionDeprecated {
                    option_name: option_name.to_string(),
                    product_version: product_version.to_string(),
                    deprecated_version: deprecated_since_version.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Check if option value fits the provided datatype
    /// # Arguments
    ///
    /// * `option_name` - name of the config option (config property or environmental variable)
    /// * `option_value` - config option value to be validated
    /// * `datatype` - containing min/max bounds, units etc.
    ///
    fn check_datatype(
        &self,
        option_name: &str,
        option_value: &str,
        datatype: &Datatype,
    ) -> Result<()> {
        // check datatype: datatype matching? min / max bounds?
        match datatype {
            Datatype::Bool => {
                self.check_datatype_scalar::<bool>(option_name, option_value, &None, &None)?;
            }
            Datatype::Integer { min, max, .. } => {
                self.check_datatype_scalar::<i64>(option_name, option_value, min, max)?;
            }
            Datatype::Float { min, max, .. } => {
                self.check_datatype_scalar::<f64>(option_name, option_value, min, max)?;
            }
            Datatype::String { min, max, unit, .. } => {
                self.check_datatype_string(option_name, option_value, min, max, unit)?;
            }
            Datatype::Array { .. } => {
                // TODO: implement logic for array type
            }
        }
        Ok(())
    }

    /// Check if option value is in allowed values
    /// # Arguments
    ///
    /// * `option_name` - name of the config option (config property or environmental variable)
    /// * `option_value` - config option value to be validated
    /// * `allowed_values` - vector of allowed values
    ///
    fn check_allowed_values(
        &self,
        option_name: &str,
        option_value: &str,
        allowed_values: &Option<Vec<String>>,
    ) -> Result<()> {
        if allowed_values.is_some() {
            let allowed_values = allowed_values.clone().unwrap();
            if !allowed_values.is_empty() && !allowed_values.contains(&option_value.to_string()) {
                return Err(Error::ConfigValueNotInAllowedValues {
                    option_name: option_name.to_string(),
                    value: option_value.to_string(),
                    allowed_values,
                });
            }
        }
        Ok(())
    }

    /// Returns the provided scalar parameter value of type T (i16, i32, i64, f32, f62-..) if no parsing errors appear
    ///
    /// # Arguments
    ///
    /// * `option_name` - name of the config option (config property or environmental variable)
    /// * `option_value` - config option value to be validated
    /// * `min` - minimum value specified in config_option.data_format.min
    /// * `max` - maximum value specified in config_option.data_format.max
    ///
    fn check_datatype_scalar<T>(
        &self,
        option_name: &str,
        option_value: &str,
        min: &Option<String>,
        max: &Option<String>,
    ) -> Result<T>
    where
        T: FromStr + std::cmp::PartialOrd + Display + Copy,
    {
        // TODO: no config value available -> can be desired behavior
        if option_value.is_empty() {
            return Err(Error::ConfigValueMissing {
                option_name: option_name.to_string(),
            });
        }

        // check if config_value fits datatype
        let val: T = self.parse::<T>(option_name, option_value)?;
        // check min bound
        self.check_bound(option_name, val, min, Config::min_bound)?;
        // check max bound
        self.check_bound(option_name, val, max, Config::max_bound)?;

        Ok(val)
    }

    /// Check if value is out of min bound
    fn min_bound<T>(val: T, min: T) -> bool
    where
        T: FromStr + std::cmp::PartialOrd + Display + Copy,
    {
        val < min
    }

    /// Check if value is out of max bound
    fn max_bound<T>(val: T, min: T) -> bool
    where
        T: FromStr + std::cmp::PartialOrd + Display + Copy,
    {
        val > min
    }

    /// Check if a value is inside a certain bound
    fn check_bound<T>(
        &self,
        option_name: &str,
        value: T,
        bound: &Option<String>,
        check_out_of_bound: fn(T, T) -> bool,
    ) -> Result<T>
    where
        T: FromStr + std::cmp::PartialOrd + Display + Copy,
    {
        if let Some(bound) = bound {
            let bound: T = self.parse::<T>(option_name, bound.as_str())?;
            if check_out_of_bound(value, bound) {
                return Err(Error::ConfigValueOutOfBounds {
                    option_name: option_name.to_string(),
                    received: value.to_string(),
                    expected: bound.to_string(),
                });
            }
        }

        Ok(value)
    }

    /// Returns the provided text parameter value of type T if no parsing errors appear
    ///
    /// # Arguments
    ///
    /// * `option_name` - name of the config option (config property or environmental variable)
    /// * `option_value` - config option value to be validated
    /// * `min` - minimum value specified in config_option.data_format.min
    /// * `max` - maximum value specified in config_option.data_format.max
    /// * `unit` - provided unit to get the regular expression to parse the option_value
    ///
    fn check_datatype_string(
        &self,
        option_name: &str,
        option_value: &str,
        min: &Option<String>,
        max: &Option<String>,
        unit: &Option<String>,
    ) -> Result<String> {
        // TODO: no config value available -> can be desired behavior
        if option_value.is_empty() {
            return Err(Error::ConfigValueMissing {
                option_name: option_name.to_string(),
            });
        }
        // len of config_value
        let len: usize = option_value.len();
        // check min bound
        self.check_bound::<usize>(option_name, len, min, Config::min_bound)?;
        // check max bound
        self.check_bound::<usize>(option_name, len, max, Config::max_bound)?;

        // check unit and respective regex
        if unit.is_none() {
            return Err(Error::UnitNotProvided {
                option_name: option_name.to_string(),
            });
        }

        let unit = unit.clone().unwrap();
        match self.config_setting_units.get(unit.as_str()) {
            None => {
                return Err(Error::UnitSettingNotFound {
                    option_name: option_name.to_string(),
                    unit,
                })
            }
            Some(regex) => {
                if !regex.is_match(option_value) {
                    return Err(Error::DatatypeRegexNotMatching {
                        option_name: option_name.to_string(),
                        value: option_value.to_string(),
                    });
                }
            }
        }

        Ok(option_value.to_string())
    }

    /// Check whether options have provided dependencies and if they are contained / set in the options map
    ///
    /// # Arguments
    ///
    /// * `options` - Map with config_option names and config_option values
    ///
    fn check_dependencies(&self, options: &HashMap<String, String>) -> Result<()> {
        for (option_name, option_value) in options {
            if let Some(option) = self.config_options.get(option_name) {
                if let Some(dependencies) = &option.depends_on {
                    for dependency in dependencies {
                        for dependency_option in &dependency.option_names {
                            // check if dependency name is include in optiions
                            match options.get(&dependency_option.name) {
                                None => {
                                    return Err(Error::ConfigDependencyMissing {
                                        option_name: option_name.clone(),
                                        dependency: dependency_option.name.clone(),
                                    });
                                }
                                Some(dependency_option_value) => {
                                    // if required value provided check value as well
                                    if let Some(value) = &dependency.value {
                                        if value != dependency_option_value {
                                            return Err(Error::ConfigDependencyValueInvalid {
                                                option_name: option_name.clone(),
                                                dependency: dependency_option.name.clone(),
                                                option_value: option_value.clone(),
                                                required_value: value.clone(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Parse a value to a certain datatype and throw error if parsing not possible
    fn parse<T: FromStr>(&self, option_name: &str, to_parse: &str) -> Result<T> {
        match to_parse.parse::<T>() {
            Ok(to_parse) => Ok(to_parse),
            Err(_) => {
                return Err(Error::DatatypeNotMatching {
                    option_name: option_name.to_string(),
                    value: to_parse.to_string(),
                    datatype: std::any::type_name::<T>().to_string(),
                })
            }
        }
    }
}

/// represents the root element structure of JSON/YAML documents
#[derive(Deserialize, Debug)]
pub struct ConfigItem {
    config_settings: ConfigSetting,
    config_options: Vec<ConfigOption>,
}

/// represents config settings like unit and regex specification
#[derive(Deserialize, Debug)]
pub struct ConfigSetting {
    unit: Vec<Unit>,
}

/// represents one config entry for a given config property or environmental variable
#[derive(Deserialize, Clone, Debug)]
pub struct ConfigOption {
    option_names: Vec<OptionName>,
    default_value: Option<Vec<DefaultValue>>,
    datatype: Datatype,
    allowed_values: Option<Vec<String>>,
    as_of_version: String,
    deprecated_since: Option<String>,
    deprecated_for: Option<Vec<String>>,
    depends_on: Option<Vec<Dependency>>,
    priority: Option<Priority>,
    apply_mode: Option<ApplyMode>,
    tags: Option<Vec<String>>,
    additional_doc: Option<Vec<String>>,
    description: Option<String>,
}

/// represents (one of multiple) unique identifier for a config option depending on the type
#[derive(Deserialize, Clone, Debug)]
struct OptionName {
    name: String,
    kind: OptionKind,
}

/// represents the config unit (name corresponds to the unit type like password and a given regex)
#[derive(Deserialize, Debug)]
pub struct Unit {
    name: String,
    regex: Option<String>,
}

/// represents different config identifier types like config property, environment variable, command line parameter etc.
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
enum OptionKind {
    Conf,
    Env,
    Cli,
}

/// represents the default value a config option may have: since default values may change with different releases, optional from and to version parameters can be provided
#[derive(Deserialize, Clone, Debug)]
struct DefaultValue {
    from_version: Option<String>,
    to_version: Option<String>,
    value: String,
}

/// represents all supported data types
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum Datatype {
    Bool,
    Integer {
        min: Option<String>,
        max: Option<String>,
        unit: Option<String>,
        accepted_units: Option<Vec<String>>,
        default_unit: Option<String>,
    },
    Float {
        min: Option<String>,
        max: Option<String>,
        unit: Option<String>,
        accepted_units: Option<Vec<String>>,
        default_unit: Option<String>,
    },
    String {
        min: Option<String>,
        max: Option<String>,
        unit: Option<String>,
        accepted_units: Option<Vec<String>>,
        default_unit: Option<String>,
    },
    Array {
        unit: Option<String>,
        accepted_units: Option<Vec<String>>,
        default_unit: Option<String>,
    },
}

/// represents a dependency on another config option and (if available) a required value
/// e.g. to set ssl certificates one has to set some property use_ssl to true
#[derive(Deserialize, Clone, Debug)]
struct Dependency {
    option_names: Vec<OptionName>,
    value: Option<String>,
}

/// represents all supported priority options
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
enum Priority {
    Optional,
    Required,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Required
    }
}

/// represents how config options are applied
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
enum ApplyMode {
    Dynamic,
    Restart,
}

impl Default for ApplyMode {
    fn default() -> Self {
        ApplyMode::Restart
    }
}

#[cfg(test)]
mod tests {
    macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map = ::std::collections::HashMap::new();
         $( map.insert($key, $val); )*
         map
    }}
}

    use crate::reader::ConfigJsonReader;
    use crate::{Config, Error};
    use rstest::*;
    use std::collections::HashMap;

    static ENV_VAR_INTEGER_PORT_MIN_MAX: &str = "ENV_VAR_INTEGER_PORT_MIN_MAX";
    static CONF_PROPERTY_STRING_MEMORY: &str = "conf.property.string.memory";
    static CONF_PROPERTY_STRING_DEPRECATED: &str = "conf.property.string.deprecated";
    static ENV_VAR_ALLOWED_VALUES: &str = "ENV_VAR_ALLOWED_VALUES";

    #[rstest(
    product_version, option_name, option_value, expected,
        case("1.0.0", ENV_VAR_INTEGER_PORT_MIN_MAX, "1000", Ok(String::from("1000"))),
        // test data type
        case("1.0.0", ENV_VAR_INTEGER_PORT_MIN_MAX, "abc", Err(Error::DatatypeNotMatching{ option_name: ENV_VAR_INTEGER_PORT_MIN_MAX.to_string(), value: "abc".to_string(), datatype: "i64".to_string() })),
        // test min bound
        case("1.0.0", ENV_VAR_INTEGER_PORT_MIN_MAX, "-1", Err(Error::ConfigValueOutOfBounds{ option_name: ENV_VAR_INTEGER_PORT_MIN_MAX.to_string(), received: "-1".to_string(), expected: "0".to_string() })),
        // test max bound
        case("1.0.0", ENV_VAR_INTEGER_PORT_MIN_MAX, "100000", Err(Error::ConfigValueOutOfBounds{ option_name: ENV_VAR_INTEGER_PORT_MIN_MAX.to_string(), received: "100000".to_string(), expected: "65535".to_string() })),
        // check version not supported
        case("0.1.0", ENV_VAR_INTEGER_PORT_MIN_MAX, "1000", Err(Error::VersionNotSupported{ option_name: ENV_VAR_INTEGER_PORT_MIN_MAX.to_string(), product_version: "0.1.0".to_string(), required_version: "0.5.0".to_string() })),
        case("0.5.0", ENV_VAR_INTEGER_PORT_MIN_MAX, "1000", Ok(String::from("1000"))),

        // check regex
        case("1.0.0", CONF_PROPERTY_STRING_MEMORY, "abc", Err(Error::DatatypeRegexNotMatching{ option_name: CONF_PROPERTY_STRING_MEMORY.to_string(), value: "abc".to_string() })),
        // check close regex
        case("1.0.0", CONF_PROPERTY_STRING_MEMORY, "100", Err(Error::DatatypeRegexNotMatching{ option_name: CONF_PROPERTY_STRING_MEMORY.to_string(), value: "100".to_string() })),
        case("1.0.0", CONF_PROPERTY_STRING_MEMORY, "1000m", Ok(String::from("1000m"))),
        case("1.0.0", CONF_PROPERTY_STRING_MEMORY, "100mb", Ok(String::from("100mb"))),

        // check deprecated
        case("0.5.0", CONF_PROPERTY_STRING_DEPRECATED, "1000m", Err(Error::VersionDeprecated { option_name: CONF_PROPERTY_STRING_DEPRECATED.to_string(), product_version: "0.5.0".to_string(), deprecated_version: "0.4.0".to_string() })),

        // check allowed values
        case("0.5.0", ENV_VAR_ALLOWED_VALUES, "allowed_value1", Ok(String::from("allowed_value1"))),
        case("0.5.0", ENV_VAR_ALLOWED_VALUES, "abc", Err(Error::ConfigValueNotInAllowedValues{ option_name: ENV_VAR_ALLOWED_VALUES.to_string(), value: "abc".to_string(), allowed_values: vec![String::from("allowed_value1"), String::from("allowed_value2"), String::from("allowed_value3")] })),
        ::trace
    )]
    fn test_validate(
        product_version: &str,
        option_name: &str,
        option_value: &str,
        expected: Result<String, Error>,
    ) {
        let reader = ConfigJsonReader::new("data/test_config.json".to_string());
        let config = Config::new(reader).unwrap();
        let result = config.validate(product_version, option_name, Some(option_value));
        assert_eq!(result, expected)
    }

    const ENV_SSL_CERTIFICATE_PATH: &str = "ENV_SSL_CERTIFICATE_PATH";
    const ENV_SSL_ENABLED: &str = "ENV_SSL_ENABLED";
    const PATH_TO_CERTIFICATE: &str = "some/path/to/certificate";

    #[rstest(
        product_version,
        options,
        expected,
        // missing dependency
        case(
            "1.0.0",
            hashmap!{
                ENV_SSL_CERTIFICATE_PATH.to_string() => "some/path/to/certificate".to_string()
            },
            Err(Error::ConfigDependencyMissing { option_name: ENV_SSL_CERTIFICATE_PATH.to_string(), dependency: "ENV_SSL_ENABLED".to_string() })
        ),
        // correct dependency
        case(
            "1.0.0",
            hashmap!{
                ENV_SSL_CERTIFICATE_PATH.to_string() => "some/path/to/certificate".to_string(),
                ENV_SSL_ENABLED.to_string() => "true".to_string()
            },
            Ok(())
        ),
        // correct dependency, wrong value
        case(
            "1.0.0",
            hashmap!{
                ENV_SSL_CERTIFICATE_PATH.to_string() => "some/path/to/certificate".to_string(),
                ENV_SSL_ENABLED.to_string() => "false".to_string()
            },
            Err(Error::ConfigDependencyValueInvalid { option_name: ENV_SSL_CERTIFICATE_PATH.to_string(), dependency: ENV_SSL_ENABLED.to_string(), option_value: PATH_TO_CERTIFICATE.to_string(), required_value: "true".to_string() })
        ),
        ::trace
    )]
    fn test_validate_all(
        product_version: &str,
        options: HashMap<String, String>,
        expected: Result<(), Error>,
    ) {
        let reader = ConfigJsonReader::new("data/test_config.json".to_string());
        let config = Config::new(reader).unwrap();
        let result = config.validate_all(product_version, &options);
        assert_eq!(result, expected)
    }
}
