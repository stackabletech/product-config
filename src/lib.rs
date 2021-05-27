//! A library to provide generalized access to a specified product configuration
//!
//! Validation of configuration properties and values in terms of:
//! - matching data types (e.g. integer, bool, string...)
//! - minimal and maximal possible values
//! - regex expressions for different units like port, url, ip etc.
//! - version and deprecated checks
//! - support for default and recommended values depending on version
//! - dependency checks for values that require other values to be set to a certain value
//! - properties can be assigned to certain rules (server, client ...)
//! - apply mode for config changes (e.g. restart)
//! - additional information like web links or descriptions
//!
pub mod error;
pub mod reader;
pub mod ser;
pub mod types;
mod util;
mod validation;

use std::collections::HashMap;
use std::str;
use std::string::String;

use crate::error::Error;
use crate::reader::ConfigReader;
use crate::types::{ProductConfigSpecProperties, PropertyName, PropertyNameKind, PropertySpec};
use crate::validation::ValidationResult;
use semver::Version;

/// This will be returned for every validated configuration value (including user values
/// and automatically added values from e.g. dependency, recommended etc.).
#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub enum PropertyValidationResult {
    /// On Default, the provided value does not differ from the default settings and may be
    /// left out from the user config in the future.
    Default(String),
    /// On RecommendedDefault, the value for this configuration property is a recommended value.
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

/// This is the main struct to hold all our knowledge about a certain product's configuration.
///
/// A product configuration consists of a list of properties and their specification
/// as well as some "configuration configuration". The latter describes some details about the configuration spec itself.
#[derive(Clone, Debug)]
pub struct ProductConfigSpec {
    // provided config units with corresponding regex pattern
    config_spec: ProductConfigSpecProperties,
    // property names as key and the corresponding property spec as value
    property_specs: HashMap<PropertyName, PropertySpec>,
}

impl ProductConfigSpec {
    /// Create a ProductConfig based on a config reader like e.g. JSON, YAML etc.
    ///
    /// # Arguments
    ///
    /// * `config_reader` - config_reader implementation
    ///
    pub fn new<CR: ConfigReader>(config_reader: CR) -> ValidationResult<Self> {
        let product_config_spec = config_reader.read()?;

        validation::validate_property_spec(
            &product_config_spec.config_spec,
            &product_config_spec.property_specs,
        )?;

        Ok(product_config_spec)
    }

    /// Retrieve and check config properties depending on the kind (e.g. env, conf),
    /// the required config file (e.g. environment variables or config properties).
    /// Add other provided properties that match the config kind, config file and config role.
    /// Automatically add and correct missing or wrong config properties and dependencies.
    ///
    /// # Arguments
    ///
    /// * `version` - the current product version
    /// * `kind` - kind provided by the user
    /// * `role` - role provided by the user
    /// * `user_config` - map with property name and values (the explicit user config properties)
    ///
    /// # Examples
    ///
    /// ```
    /// use product_config::reader::ConfigJsonReader;
    /// use product_config::types::PropertyNameKind;
    /// use product_config::ProductConfigSpec;
    /// use std::collections::HashMap;
    ///
    /// let config = ProductConfigSpec::new(ConfigJsonReader::new(
    ///     "data/test_config_spec.json",
    ///     "data/test_property_spec.json",
    ///   )
    /// ).unwrap();
    ///
    /// let mut user_data = HashMap::new();
    /// user_data.insert("ENV_INTEGER_PORT_MIN_MAX".to_string(), "12345".to_string());
    /// user_data.insert("ENV_PROPERTY_STRING_MEMORY".to_string(), "1g".to_string());
    ///
    /// let env_sh = config.get(
    ///     "0.5.0",
    ///     &PropertyNameKind::Conf("env.sh".to_string()),
    ///     Some("role_1"),
    ///     &user_data,
    /// );
    /// ```
    ///
    pub fn get(
        &self,
        version: &str,
        kind: &PropertyNameKind,
        role: Option<&str>,
        user_config: &HashMap<String, String>,
    ) -> ValidationResult<HashMap<String, PropertyValidationResult>> {
        let mut result_config = HashMap::new();

        let product_version = Version::parse(version)?;

        // merge provided user properties with extracted property spec via role / kind and
        // dependencies to be validated later.
        let merged_properties = self.merge_properties(user_config, &product_version, kind, role);

        for (name, value) in &merged_properties {
            let property_name = &PropertyName {
                name: name.clone(),
                kind: kind.clone(),
            };

            result_config.insert(
                property_name.name.clone(),
                validation::validate(
                    &self.property_specs,
                    &self.config_spec,
                    &merged_properties,
                    &product_version,
                    role,
                    property_name,
                    value,
                ),
            );
        }

        Ok(result_config)
    }

    /// Merge provided user config properties and available property spec (from JSON, YAML...)
    /// depending on kind and role to be validated later.
    ///
    /// # Arguments
    ///
    /// * `user_config` - map with property name and values (the explicit user config properties)
    /// * `version` - the current product version
    /// * `kind` - property name kind provided by the user
    /// * `role` - property role provided by the user
    ///
    fn merge_properties(
        &self,
        user_config: &HashMap<String, String>,
        version: &Version,
        kind: &PropertyNameKind,
        role: Option<&str>,
    ) -> HashMap<String, String> {
        let mut merged_properties = HashMap::new();

        if let Ok(properties) =
            util::get_matching_properties(&self.property_specs, kind, role, version)
        {
            merged_properties.extend(properties)
        }

        if let Ok(dependencies) =
            util::get_matching_dependencies(&self.property_specs, user_config, version, kind)
        {
            merged_properties.extend(dependencies);
        }

        merged_properties.extend(user_config.clone());

        merged_properties
    }
}

#[cfg(test)]
mod tests {
    use crate::error::Error;
    use crate::reader::ConfigJsonReader;
    use crate::types::{PropertyName, PropertyNameKind};
    use crate::{ProductConfigSpec, PropertyValidationResult};
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
        HashMap<String, PropertyValidationResult>,
    ) {
        let float_recommended = "50.0";
        let port_recommended = "20000";

        let data = HashMap::new();

        let mut expected = HashMap::new();
        expected.insert(
            ENV_INTEGER_PORT_MIN_MAX.to_string(),
            PropertyValidationResult::RecommendedDefault(port_recommended.to_string()),
        );
        expected.insert(
            ENV_FLOAT.to_string(),
            PropertyValidationResult::RecommendedDefault(float_recommended.to_string()),
        );
        (data, expected)
    }

    fn create_correct_data_and_expected() -> (
        HashMap<String, String>,
        HashMap<String, PropertyValidationResult>,
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
            PropertyValidationResult::Valid(port.to_string()),
        );
        expected.insert(
            ENV_SSL_CERTIFICATE_PATH.to_string(),
            PropertyValidationResult::Valid(certificate_path.to_string()),
        );
        expected.insert(
            ENV_SSL_ENABLED.to_string(),
            PropertyValidationResult::RecommendedDefault(ssl_enabled.to_string()),
        );
        expected.insert(
            ENV_FLOAT.to_string(),
            PropertyValidationResult::Valid(float_value.to_string()),
        );

        (data, expected)
    }

    #[rstest]
    #[case(
        VERSION_0_5_0,
        &PropertyNameKind::Conf(CONF_FILE.to_string()),
        Some(ROLE_1),
        create_empty_data_and_expected().0,
        create_empty_data_and_expected().1,
    )]
    #[case(
      VERSION_0_5_0,
      &PropertyNameKind::Conf(CONF_FILE.to_string()),
      Some(ROLE_1),
      create_correct_data_and_expected().0,
      create_correct_data_and_expected().1,
    )]
    #[trace]
    fn test_get_kind_conf_role_1(
        #[case] version: &str,
        #[case] kind: &PropertyNameKind,
        #[case] role: Option<&str>,
        #[case] user_data: HashMap<String, String>,
        #[case] expected: HashMap<String, PropertyValidationResult>,
    ) {
        let config = ProductConfigSpec::new(ConfigJsonReader::new(
            "data/test_config_spec.json",
            "data/test_property_spec.json",
        ))
        .unwrap();

        let result = config.get(version, kind, role, &user_data).unwrap();

        println!("Size: {}", result.len());
        for x in &result {
            println!("{:?}", x)
        }

        assert_eq!(result, expected);
    }

    #[test]
    fn test_product_config_result_order() {
        let valid = PropertyValidationResult::Valid("valid".to_string());
        let default = PropertyValidationResult::Default("default".to_string());
        let recommended = PropertyValidationResult::RecommendedDefault("recommended".to_string());
        let warn = PropertyValidationResult::Warn(
            "warning".to_string(),
            Error::PropertyNotFound {
                property_name: PropertyName {
                    name: "test".to_string(),
                    kind: PropertyNameKind::Conf("my_config".to_string()),
                },
            },
        );
        let error = PropertyValidationResult::Error(Error::ConfigSpecPropertiesNotFound {
            name: "xyz".to_string(),
        });

        assert!(valid > recommended);
        assert!(valid > default);
        assert!(valid < error);

        assert!(warn < error);
        assert!(error > warn);
    }
}
