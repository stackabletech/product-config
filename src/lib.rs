mod config_reader;

extern crate lazy_static;

use serde::Deserialize;
use std::collections::HashMap;
use std::str;
use std::str::FromStr;
use std::string::String;

use regex::Regex;
use std::fmt::Display;
use thiserror::Error;
use crate::config_reader::ConfigReader;

#[derive(Debug)]
pub struct Config {
  // provided config units with regex
  pub config_setting_units: HashMap<String, Regex>,
  // conf property or env variable name as key and the corresponding option as value
  pub config_options: HashMap<String, ConfigOption>,
}

impl Config {
  /// Returns a Config with data from the loaded from the config reader
  ///
  /// # Arguments
  ///
  /// * `config_reader` - A config reader implementing the trait ConfigReader
  ///
  /// # Examples
  ///
  /// ```
  /// use product_config::{Config, ConfigJsonReader};
  /// path_to_config = String::from("path/to/config.json");
  /// config_reader = ConfigJsonReader::new(path_to_config);
  /// config = Config::new(config_reader);
  /// ```
  pub fn new<'a, CR: ConfigReader<'a ,ConfigItem>>(config_reader: CR) -> Self {
    let config: ConfigItem = config_reader.read().unwrap();
    // convert config options
    let mut config_options: HashMap<String, ConfigOption> = HashMap::new();
    let mut config_option_name: String;

    for opt in config.config_options {
      if !opt.env.is_none() && opt.env != Some(String::from("")) {
        config_option_name = opt.env.clone().unwrap();
      } else if !opt.property.is_none() && opt.property != Some(String::from("")) {
        config_option_name = opt.property.clone().unwrap();
      } else {
        // TODO: or just skip?
        panic!(
          "No config property or environment variable provided in: {:?}",
          opt
        );
      }
      config_options.insert(config_option_name, opt);
    }

    // convert settings
    let mut config_setting_units: HashMap<String, Regex> = HashMap::new();
    let mut config_setting_unit_name: String;

    for unit in config.config_setting.unit {
      if !unit.name.is_empty() {
        config_setting_unit_name = unit.name;
      } else {
        // TODO: or just skip?
        panic!("No unit provided in: {:?}", unit);
      }

      let config_setting_unit_regex = unit.regex.clone().unwrap_or("".to_string());
      let regex = match Regex::new(config_setting_unit_regex.as_str()) {
        Ok(regex) => regex,
        Err(error) => {
          panic!(
            "Unit[{}] -> could not create regex -> {}",
            config_setting_unit_name, error
          );
        }
      };

      config_setting_units.insert(config_setting_unit_name, regex);
    }

    Config {
      config_setting_units,
      config_options,
    }
  }

  /// Returns the provided config_option_value if no validation errors appear
  ///
  /// # Arguments
  ///
  /// * `product_version` - version of the currently active product version
  /// * `config_option_name` - name of the config option (config property or environmental variable)
  /// * `config_option_value` - config option value to be validated
  ///
  /// # Examples
  ///
  /// ```
  /// use product_config::{Config, ConfigError};
  /// path_to_config = "path/to/config.json";
  /// config_reader = ConfigJsonReader::new(path_to_config);
  /// config = Config::new(config_reader);
  /// match config::validate("some_product_version","some_config_property","some_config_property_value") {
  ///   Ok(_) => {}
  ///   Err(e) => match e {
  ///     ConfigError::ConfigValueNotFound(_) => {}
  ///     ConfigError::ConfigVersionNotSupported(_, _, _) => {}
  ///     ConfigError::ConfigVersionDeprecated(_, _, _) => {}
  ///     ConfigError::ConfigValueMinOutOfBounds(_, _, _) => {}
  ///     ConfigError::ConfigValueMaxOutOfBounds(_, _, _) => {}
  ///     ConfigError::ConfigValueEmptyOrNone(_) => {}
  ///     ConfigError::ConfigValueNotInAllowedValues(_, _, _) => {}
  ///     ConfigError::DataFormatTypeNotParsable(_, _, _) => {}
  ///     ConfigError::DataFormatNoUnitProvided(_) => {}
  ///     ConfigError::DataFormatNoRegexMatch(_, _) => {}
  ///   }
  /// }
  /// ```
  pub fn validate(
    &self,
    product_version: &str,
    config_option_name: &str,
    config_option_value: &str,
  ) -> Result<String, ConfigError> {
    // name not found
    if !self.config_options.contains_key(config_option_name) {
      return Err(ConfigError::ConfigValueNotFound(
        config_option_name.to_string(),
      ));
    }

    let option = self.config_options.get(config_option_name).unwrap();

    // compare version
    if option.as_of_version > product_version.to_string() {
      return Err(ConfigError::ConfigVersionNotSupported(
        config_option_name.to_string(),
        product_version.to_string(),
        option.as_of_version.to_string(),
      ));
    }
    // check if deprecated
    let deprecated_since = option.deprecated_since.as_deref().unwrap_or("");
    if !deprecated_since.is_empty() && deprecated_since <= product_version {
      return Err(ConfigError::ConfigVersionDeprecated(
        config_option_name.to_string(),
        product_version.to_string(),
        deprecated_since.to_string(),
      ));
    }
    // check data type and min / max
    let data_format_min = option.data_format.min.as_deref().unwrap_or("");
    let data_format_max = option.data_format.max.as_deref().unwrap_or("");

    match option.data_format.datatype {
      Datatype::Bool => {
        Config::check_datatype_scalar::<bool>(
          config_option_name,
          config_option_value,
          data_format_min,
          data_format_max,
        )?;
      }
      Datatype::Integer => {
        Config::check_datatype_scalar::<i64>(
          config_option_name,
          config_option_value,
          data_format_min,
          data_format_max,
        )?;
      }
      Datatype::Float => {
        Config::check_datatype_scalar::<f64>(
          config_option_name,
          config_option_value,
          data_format_min,
          data_format_max,
        )?;
      }
      Datatype::String => {
        let unit = option.data_format.unit.clone().unwrap_or("".to_string());
        // check unit
        if unit.is_empty() || !self.config_setting_units.contains_key(unit.as_str()) {
          return Err(ConfigError::DataFormatNoUnitProvided(
            config_option_name.to_string(),
          ));
        }
        // check min / max str length and for regex match
        Config::check_datatype_string(
          config_option_name,
          config_option_value,
          data_format_min,
          data_format_max,
          self.config_setting_units.get(unit.as_str()).unwrap(),
        )?;
      }
      Datatype::Array => {
        // TODO: implement logic for array type
      }
    }

    // check allowed values
    if !option.allowed_values.is_none() {
      let allowed_values = option.allowed_values.as_ref().unwrap();
      if !allowed_values.is_empty() && !allowed_values.contains(&config_option_value.to_string())
      {
        return Err(ConfigError::ConfigValueNotInAllowedValues(
          config_option_name.to_string(),
          config_option_value.to_string(),
          format!("{:?}", allowed_values),
        ));
      }
    }

    Ok(config_option_value.to_string())
  }

  /// Returns the provided scalar parameter value of type T (i16, i32, i64, f32, f62-..) if no parsing errors appear
  ///
  /// # Arguments
  ///
  /// * `config_option_name` - name of the config option (config property or environmental variable)
  /// * `config_option_value` - config option value to be validated
  /// * `data_format_min` - minimum value specified in config_option.data_format.min
  /// * `data_format_max` - maximum value specified in config_option.data_format.max
  fn check_datatype_scalar<T>(
    config_option_name: &str,
    config_option_value: &str,
    data_format_min: &str,
    data_format_max: &str,
  ) -> Result<T, ConfigError>
    where
      T: FromStr + std::cmp::PartialOrd + Display,
  {
    // no config value available
    if config_option_value.is_empty() {
      return Err(ConfigError::ConfigValueEmptyOrNone(config_option_name.to_string()));
    }
    // check if config_value fits datatype
    let val: T = Config::parse::<T>(config_option_name, config_option_value)?;
    // min available
    if !data_format_min.is_empty() {
      // check if max fits datatype
      let min = Config::parse::<T>(config_option_name, data_format_min)?;
      if val < min {
        return Err(ConfigError::ConfigValueMinOutOfBounds(
          config_option_name.to_string(),
          val.to_string(),
          min.to_string(),
        ));
      }
    }
    // max available
    if !data_format_max.is_empty() {
      // check if max fits datatype
      let max = Config::parse::<T>(config_option_name, data_format_max)?;
      if val > max {
        return Err(ConfigError::ConfigValueMaxOutOfBounds(
          config_option_name.to_string(),
          val.to_string(),
          max.to_string(),
        ));
      }
    }

    Ok(val)
  }

  /// Returns the provided text parameter value of type T if no parsing errors appear
  ///
  /// # Arguments
  ///
  /// * `config_option_name` - name of the config option (config property or environmental variable)
  /// * `config_option_value` - config option value to be validated
  /// * `data_format_min` - minimum value specified in config_option.data_format.min
  /// * `data_format_max` - maximum value specified in config_option.data_format.max
  /// * `regex` - regular expression provided by the specified unit to parse the config_option_value
  fn check_datatype_string(
    config_option_name: &str,
    config_option_value: &str,
    data_format_min: &str,
    data_format_max: &str,
    regex: &Regex,
  ) -> Result<String, ConfigError> {
    // no config value available
    if config_option_value.is_empty() {
      return Err(ConfigError::ConfigValueEmptyOrNone(config_option_name.to_string()));
    }
    // len of config_value
    let len: usize = config_option_value.len();
    // min available
    if !data_format_min.is_empty() {
      // check if max fits datatype
      let min = Config::parse::<usize>(config_option_name, data_format_min)?;
      if len < min {
        return Err(ConfigError::ConfigValueMinOutOfBounds(
          config_option_name.to_string(),
          len.to_string(),
          min.to_string(),
        ));
      }
    }
    // max available
    if !data_format_max.is_empty() {
      // check if max fits datatype
      let max = Config::parse::<usize>(config_option_name, data_format_max)?;
      if len > max {
        return Err(ConfigError::ConfigValueMaxOutOfBounds(
          config_option_name.to_string(),
          len.to_string(),
          max.to_string(),
        ));
      }
    }
    // regex
    if !regex.is_match(config_option_value) {
      return Err(ConfigError::DataFormatNoRegexMatch(
        config_option_name.to_string(),
        config_option_value.to_string(),
      ));
    }

    Ok(config_option_value.to_string())
  }

  fn parse<T: FromStr>(config_option_name: &str, to_parse: &str) -> Result<T, ConfigError> {
    match to_parse.parse::<T>() {
      Ok(to_parse) => Ok(to_parse),
      Err(_) => {
        return Err(ConfigError::DataFormatTypeNotParsable(
          config_option_name.to_string(),
          to_parse.to_string(),
          std::any::type_name::<T>().to_string(),
        ))
      }
    }
  }
}

/// error definitions
#[derive(PartialEq, Error, Debug)]
pub enum ConfigError {
  #[error("[{0}]: no config available")]
  ConfigValueNotFound(String),

  #[error("[{0}]: current controller version is [{1}] -> option available since version [{2}]")]
  ConfigVersionNotSupported(String, String, String),

  #[error("[{0}]: current controller version is [{1}] -> option deprecated since version [{2}]")]
  ConfigVersionDeprecated(String, String, String),

  #[error("[{0}]: value/size[{1}] < expected minimum[{2}]")]
  ConfigValueMinOutOfBounds(String, String, String),

  #[error("[{0}]: value/size[{1}] > expected maximum[{2}]")]
  ConfigValueMaxOutOfBounds(String, String, String),

  #[error("[{0}]: provided config value empty")]
  ConfigValueEmptyOrNone(String),

  #[error("[{0}]: value [{1}] not in allowed values: {2:?}")]
  ConfigValueNotInAllowedValues(String, String, String),

  #[error("[{0}]: value[{1}] not of specified type: [{2}]")]
  DataFormatTypeNotParsable(String, String, String),

  #[error("[{0}]: missing unit")]
  DataFormatNoUnitProvided(String),

  #[error("[{0}] -> value[{1}] does not fit regex")]
  DataFormatNoRegexMatch(String, String),
}

/// represents the root element structure of JSON/YAML documents
#[derive(Deserialize, Debug)]
pub struct ConfigItem {
  config_setting: ConfigSetting,
  config_options: Vec<ConfigOption>,
}

/// represents config settings like unit and regex specification
#[derive(Deserialize, Debug)]
pub struct ConfigSetting {
  unit: Vec<ConfigUnit>,
}

/// represents the config unit (name corresponds to the unit type like password and a given regex)
#[derive(Deserialize, Debug)]
pub struct ConfigUnit {
  name: String,
  regex: Option<String>,
}

/// represents one config entry for a given config property or environmental variable
#[derive(Deserialize, Debug)]
pub struct ConfigOption {
  property: Option<String>,
  env: Option<String>,
  default_value: Option<Vec<DefaultValue>>,
  data_format: DataFormat,
  allowed_values: Option<Vec<String>>,
  as_of_version: String,
  deprecated_since: Option<String>,
  deprecated_for: Option<Vec<String>>,
  importance: Option<Importance>,
  tags: Option<Vec<String>>,
  additional_doc: Option<Vec<String>>,
  description: Option<String>,
}

/// represents the default value a config option may have: since default values may change with different releases, optional from and to version parameters can be provided
#[derive(Deserialize, Debug)]
struct DefaultValue {
  from_version: Option<String>,
  to_version: Option<String>,
  value: String,
}

/// represents the data format a config option may have
#[derive(Deserialize, Debug)]
struct DataFormat {
  datatype: Datatype,
  min: Option<String>,
  max: Option<String>,
  unit: Option<String>,
  accepted_units: Option<Vec<String>>,
  default_unit: Option<String>,
}

/// represents all supported data types
#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum Datatype {
  Bool,
  Integer,
  Float,
  String,
  Array,
}

/// represents all supported "importance" parameters
#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum Importance {
  Nullable,
  Optional,
  Required,
}

#[cfg(test)]
mod tests {
  use crate::config_reader::ConfigJsonReader;
  use crate::{Config, ConfigError};
  use rstest::*;

  lazy_static! {
    static ref CONFIG: Config = Config::new(ConfigJsonReader::new("data/test_config.json".to_string()));
  }

  static ENV_VAR_INTEGER_PORT_MIN_MAX: &str = "ENV_VAR_INTEGER_PORT_MIN_MAX";
  static CONF_PROPERTY_STRING_MEMORY :&str = "conf.property.string.memory";
  static CONF_PROPERTY_STRING_DEPRECATED: &str = "conf.property.string.deprecated";
  static ENV_VAR_ALLOWED_VALUES: &str = "ENV_VAR_ALLOWED_VALUES";

  #[rstest(
  product_version, config_option_name, config_option_value, expected,
  case("1.0.0", ENV_VAR_INTEGER_PORT_MIN_MAX, "1000", Ok(String::from("1000"))),
  // test data type
  case("1.0.0", ENV_VAR_INTEGER_PORT_MIN_MAX, "abc", Err(ConfigError::DataFormatTypeNotParsable(ENV_VAR_INTEGER_PORT_MIN_MAX.to_string(), "abc".to_string(), "i64".to_string()))),
  // test min bound
  case("1.0.0", ENV_VAR_INTEGER_PORT_MIN_MAX, "-1", Err(ConfigError::ConfigValueMinOutOfBounds(ENV_VAR_INTEGER_PORT_MIN_MAX.to_string(), "-1".to_string(), "0".to_string()))),
  // test max bound
  case("1.0.0", ENV_VAR_INTEGER_PORT_MIN_MAX, "100000", Err(ConfigError::ConfigValueMaxOutOfBounds(ENV_VAR_INTEGER_PORT_MIN_MAX.to_string(), "100000".to_string(), "65535".to_string()))),
  // check version not supported
  case("0.1.0", ENV_VAR_INTEGER_PORT_MIN_MAX, "1000", Err(ConfigError::ConfigVersionNotSupported(ENV_VAR_INTEGER_PORT_MIN_MAX.to_string(), "0.1.0".to_string(), "0.5.0".to_string()))),
  case("0.5.0", ENV_VAR_INTEGER_PORT_MIN_MAX, "1000", Ok(String::from("1000"))),

  // check regex
  case("1.0.0", CONF_PROPERTY_STRING_MEMORY, "abc", Err(ConfigError::DataFormatNoRegexMatch(CONF_PROPERTY_STRING_MEMORY.to_string(), "abc".to_string()))),
  // check close regex
  case("1.0.0", CONF_PROPERTY_STRING_MEMORY, "100", Err(ConfigError::DataFormatNoRegexMatch(CONF_PROPERTY_STRING_MEMORY.to_string(), "100".to_string()))),
  case("1.0.0", CONF_PROPERTY_STRING_MEMORY, "1000m", Ok(String::from("1000m"))),
  case("1.0.0", CONF_PROPERTY_STRING_MEMORY, "100mb", Ok(String::from("100mb"))),

  // check deprecated
  case("0.5.0", CONF_PROPERTY_STRING_DEPRECATED, "1000m", Err(ConfigError::ConfigVersionDeprecated(CONF_PROPERTY_STRING_DEPRECATED.to_string(), "0.5.0".to_string(), "0.4.0".to_string()))),

  // check allowed values
  case("0.5.0", ENV_VAR_ALLOWED_VALUES, "allowed_value1", Ok(String::from("allowed_value1"))),
  case("0.5.0", ENV_VAR_ALLOWED_VALUES, "abc", Err(ConfigError::ConfigValueNotInAllowedValues(ENV_VAR_ALLOWED_VALUES.to_string(), "abc".to_string(), "[\"allowed_value1\", \"allowed_value2\", \"allowed_value3\"]".to_string())))
  ::trace
  )]
  fn test_data_format(product_version: &str, config_option_name: &str, config_option_value: &str, expected: Result<String, ConfigError>) {
    let result = CONFIG.validate(product_version, config_option_name, config_option_value);
    assert_eq!(result, expected)
  }

}
