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
use std::collections::{BTreeMap, HashMap};
use std::string::String;
use std::{fs, str};

use semver::Version;

use crate::error::Error;
use crate::types::{ProductConfig, PropertyName, PropertyNameKind, PropertySpec};
use crate::util::semver_parse;
use crate::validation::ValidationResult;
use std::ops::Deref;

pub mod error;
pub mod ser;
pub mod types;
pub mod writer;

mod util;
mod validation;

pub struct ProductConfigManager {
    config: ProductConfig,
}

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
    /// On Override the given property name does not exist in the product config, and therefore
    /// no checks could be applied for the value.
    Override(String),
    /// On warn, the value maybe used with caution.
    Warn(String, Error),
    /// On error, check the provided config and config values.
    /// Should never be used like this!
    Error(String, Error),
}

impl ProductConfigManager {
    /// Create a ProductConfig from a YAML file.
    ///
    /// # Arguments
    ///
    /// * `file_path` - the path to the YAML file
    pub fn from_yaml_file(file_path: &str) -> ValidationResult<Self> {
        let contents = fs::read_to_string(file_path).map_err(|_| error::Error::FileNotFound {
            file_name: file_path.to_string(),
        })?;

        Ok(ProductConfigManager {
            config: serde_yaml::from_str(&contents).map_err(|serde_error| {
                error::Error::FileNotParsable {
                    file_name: file_path.to_string(),
                    reason: serde_error.to_string(),
                }
            })?,
        })
    }

    /// Retrieve and check config properties depending on the kind (e.g. env, conf),
    /// the required config file (e.g. environment variables or config properties).
    /// Add other provided properties that match the config kind, config file and config role.
    /// Automatically add and correct missing or wrong config properties and dependencies.
    ///
    /// # Arguments
    ///
    /// * `version` - the current product version
    /// * `role` - role provided by the user
    /// * `kind` - kind provided by the user
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
    ///     "role_1",
    ///     &PropertyNameKind::File("env.sh".to_string()),
    ///     &user_data,
    /// );
    /// ```
    pub fn get(
        &self,
        version: &str,
        role: &str,
        kind: &PropertyNameKind,
        user_config: HashMap<String, Option<String>>,
    ) -> BTreeMap<String, PropertyValidationResult> {
        let mut result_config: BTreeMap<String, PropertyValidationResult> = BTreeMap::new();

        let product_version = semver_parse(version).unwrap();

        // merge provided user properties with extracted property spec via role / kind and
        // dependencies to be validated later.
        let mut merged_properties = self
            .get_and_expand_properties(&product_version, role, kind)
            .unwrap();

        merged_properties.extend(user_config);

        self.validate(&product_version, role, kind, merged_properties)
    }

    /// Merge provided user config properties and available property spec (from JSON, YAML...)
    /// depending on kind and role to be validated later.
    ///
    /// # Arguments
    ///
    /// * `version` - the current product version
    /// * `role` - property role provided by the user
    /// * `kind` - property name kind provided by the user
    pub fn get_and_expand_properties(
        &self,
        version: &Version,
        role: &str,
        kind: &PropertyNameKind,
    ) -> ValidationResult<BTreeMap<String, Option<String>>> {
        let mut merged_properties = BTreeMap::new();

        for property in &self.config.properties {
            if !property.has_role_required(role) {
                continue;
            }

            if !property.is_version_supported(version)? {
                continue;
            }

            if let Some((name, value)) = property.recommended_or_default(version, kind) {
                merged_properties.insert(name, value);
            }

            if let Some(expands_to) = &property.expands_to {
                for dependency in expands_to {
                    if !dependency.property.has_role(role) {
                        continue;
                    }

                    if !dependency.property.is_version_supported(version)? {
                        continue;
                    }

                    if let Some(name) = dependency.property.name_from_kind(kind) {
                        if dependency.value.is_some() {
                            merged_properties.insert(name, dependency.value.clone());
                        } else if let Some((_, value)) =
                            dependency.property.recommended_or_default(version, kind)
                        {
                            merged_properties.insert(name, value);
                        }
                    }
                }
            }
        }

        Ok(merged_properties)
    }

    /// Returns the provided property_value if no validation errors appear
    ///
    /// # Arguments
    /// * `version` - the current product version
    /// * `role` - property role provided by the user
    /// * `kind` - property name kind provided by the user
    /// * `merged_properties` - merged user and property spec (matching role, kind etc.)
    pub fn validate(
        &self,
        version: &Version,
        role: &str,
        kind: &PropertyNameKind,
        merged_properties: BTreeMap<String, Option<String>>,
    ) -> BTreeMap<String, PropertyValidationResult> {
        let mut result = BTreeMap::new();

        for (name, value) in merged_properties {
            match value {
                Some(val) => {
                    let property = match self.look_up_property(&name, role, kind, version) {
                        Some(property) => property,
                        None => {
                            result
                                .insert(name, PropertyValidationResult::Override(val.to_string()));
                            continue;
                        }
                    };

                    let check_datatype = validation::check_datatype(&property, &name, &val);
                    if check_datatype.is_err() {
                        result.insert(
                            name.to_string(),
                            PropertyValidationResult::Error(
                                val.to_string(),
                                check_datatype.err().unwrap(),
                            ),
                        );
                        continue;
                    }

                    // TODO: deprecated check

                    // value is valid, check if it matches recommended or default values
                    // was provided by recommended value?
                    if let Some(recommended) = &property.recommended_values {
                        let recommended_value =
                            property.filter_value(version, recommended.as_slice());
                        if recommended_value == Some(val.to_string()) {
                            result.insert(
                                name.to_string(),
                                PropertyValidationResult::RecommendedDefault(val.to_string()),
                            );
                            continue;
                        }
                    }

                    // was provided by recommended value?
                    if let Some(default) = &property.default_values {
                        let default_value = property.filter_value(version, default.as_slice());
                        if default_value == Some(val.to_string()) {
                            result.insert(
                                name.to_string(),
                                PropertyValidationResult::Default(val.to_string()),
                            );
                            continue;
                        }
                    }

                    result.insert(
                        name.to_string(),
                        PropertyValidationResult::Valid(val.to_string()),
                    );
                }
                None => {
                    // TODO: we currently do not have nullable field in the spec; if we add this
                    //    we can perform the nullable check here.
                    result.insert(
                        name.to_string(),
                        PropertyValidationResult::Valid("".to_string()),
                    );
                }
            }
        }

        result
    }

    pub fn look_up_property(
        &self,
        name: &str,
        role: &str,
        kind: &PropertyNameKind,
        version: &Version,
    ) -> Option<PropertySpec> {
        for propertyAnchor in &self.config.properties {
            if propertyAnchor.name_from_kind(kind) != Some(name.to_string()) {
                continue;
            }

            if !propertyAnchor.has_role(role) {
                continue;
            }

            if propertyAnchor.is_version_supported(version).is_err() {
                continue;
            }

            return Some(propertyAnchor.property.clone());
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};

    use super::*;
    use crate::error::Error;
    use crate::types::{PropertyName, PropertyNameKind};
    use crate::util::semver_parse;
    use crate::ProductConfigManager;
    use crate::PropertyValidationResult;
    use rstest::*;

    const ENV_INTEGER_PORT_MIN_MAX: &str = "ENV_INTEGER_PORT_MIN_MAX";

    const ENV_FLOAT: &str = "ENV_FLOAT";
    //const ENV_PROPERTY_STRING_MEMORY: &str = "ENV_PROPERTY_STRING_MEMORY";
    const ENV_PROPERTY_STRING_DEPRECATED: &str = "ENV_PROPERTY_STRING_DEPRECATED";
    //const ENV_ALLOWED_VALUES: &str = "ENV_ALLOWED_VALUES";
    //const ENV_SECURITY: &str = "ENV_SECURITY";
    //const ENV_SECURITY_PASSWORD: &str = "ENV_SECURITY_PASSWORD";
    const ENV_SSL_ENABLED: &str = "ENV_SSL_ENABLED";
    const ENV_SSL_CERTIFICATE_PATH: &str = "ENV_SSL_CERTIFICATE_PATH";

    const ROLE_1: &str = "role_1";
    const VERSION_0_5_0: &str = "0.5.0";
    const CONF_FILE: &str = "env.sh";

    fn create_empty_data_and_expected() -> (
        HashMap<String, Option<String>>,
        BTreeMap<String, PropertyValidationResult>,
    ) {
        let float_recommended = "50.0";
        let port_recommended = "20000";

        let data = HashMap::new();

        let mut expected = BTreeMap::new();
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
        HashMap<String, Option<String>>,
        BTreeMap<String, PropertyValidationResult>,
    ) {
        let port = "12345";
        let ssl_enabled = "true";
        let certificate_path = "/tmp/ssl_key.xyz";
        let float_value = "55.555";

        let mut data = HashMap::new();
        data.insert(ENV_INTEGER_PORT_MIN_MAX.to_string(), Some(port.to_string()));
        data.insert(
            ENV_SSL_CERTIFICATE_PATH.to_string(),
            Some(certificate_path.to_string()),
        );
        data.insert(ENV_FLOAT.to_string(), Some(float_value.to_string()));

        let mut expected = BTreeMap::new();

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
        &PropertyNameKind::File(CONF_FILE.to_string()),
        ROLE_1,
        create_empty_data_and_expected().0,
        create_empty_data_and_expected().1,
    )]
    #[case(
      VERSION_0_5_0,
      &PropertyNameKind::File(CONF_FILE.to_string()),
      ROLE_1,
      create_correct_data_and_expected().0,
      create_correct_data_and_expected().1,
    )]
    #[trace]
    fn test_get_kind_conf_role_1(
        #[case] version: &str,
        #[case] kind: &PropertyNameKind,
        #[case] role: &str,
        #[case] user_data: HashMap<String, Option<String>>,
        #[case] expected: BTreeMap<String, PropertyValidationResult>,
    ) {
        let manager =
            ProductConfigManager::from_yaml_file("data/test_product_config.yaml").unwrap();

        let result = manager.get(version, role, kind, user_data);

        println!("Size: {}", result.len());
        for x in &result {
            println!("{:?}", x)
        }

        assert_eq!(result, expected);
    }

    #[test]
    fn test_product_config_manager_merge_user_and_config_properties() {
        let manager =
            ProductConfigManager::from_yaml_file("data/test_product_config.yaml").unwrap();

        let mut user_config = HashMap::new();
        user_config.insert(
            ENV_INTEGER_PORT_MIN_MAX.to_string(),
            Some("5000".to_string()),
        );
        user_config.insert(ENV_FLOAT.to_string(), Some("5.888".to_string()));
        user_config.insert(
            ENV_SSL_CERTIFICATE_PATH.to_string(),
            Some("a/b/c".to_string()),
        );

        let mut expected = BTreeMap::new();
        // vaild, expected
        expected.insert(
            ENV_INTEGER_PORT_MIN_MAX.to_string(),
            Some("5000".to_string()),
        );
        // valid, expected
        expected.insert(ENV_FLOAT.to_string(), Some("5.888".to_string()));
        // expected
        expected.insert(
            ENV_PROPERTY_STRING_DEPRECATED.to_string(),
            Some("".to_string()),
        );
        //ENV_PROPERTY_STRING_DEPRECATED PropertyValidationResult::Error()
        // required but no recommended or default value: expected
        //ENV_SECURITY_PASSWORD PropertyValidationResult::Error()
        // dependency of ENV_SECURITY_PASSWORD: not expected
        //ENV_SECURITY true
        // valid, expected
        //ENV_SSL_CERTIFICATE_PATH "a/b/c"
        // expected
        //ENV_SSL_ENABLED "true"

        let got = manager
            .get_and_expand_properties(
                &semver_parse(VERSION_0_5_0).unwrap(),
                ROLE_1,
                &PropertyNameKind::File(CONF_FILE.to_string()),
            )
            .unwrap();

        assert_eq!(expected, got);
    }
}
