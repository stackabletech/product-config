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
use crate::util::{expand_properties, semver_parse};
use crate::validation::{check_allowed_values, ValidationResult};
use std::str::FromStr;

pub mod error;
pub mod ser;
pub mod types;
pub mod writer;

mod util;
mod validation;

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
    /// On Unknown the given property name does not exist in the product config, and therefore
    /// no checks could be applied for the value.
    Unknown(String),
    /// On warn, the value maybe used with caution.
    Warn(String, Error),
    /// On error, check the provided config and config values.
    /// Should never be used like this!
    Error(String, Error),
}

/// The struct to interact with the product config. Reads and parses a YAML product configuration.
/// Performs validation and merging task with user defined properties and the properties provided
/// in the YAML product configuration.
pub struct ProductConfigManager {
    config: ProductConfig,
}

impl FromStr for ProductConfigManager {
    type Err = error::Error;
    /// Create a ProductConfig from a YAML string.
    ///
    /// # Arguments
    ///
    /// * `contents` - the YAML string content
    fn from_str(contents: &str) -> ValidationResult<Self> {
        Ok(ProductConfigManager {
            config: serde_yaml::from_str(contents).map_err(|serde_error| {
                error::Error::YamlNotParsable {
                    content: contents.to_string(),
                    reason: serde_error.to_string(),
                }
            })?,
        })
    }
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

        Self::from_str(&contents).map_err(|serde_error| error::Error::YamlFileNotParsable {
            file: file_path.to_string(),
            reason: serde_error.to_string(),
        })
    }

    /// This function merges the user provided configuration properties with the product configuration
    /// and validates the result, both in a single step. The caller is expected to look at each
    /// [PropertyValidationResult] and take the appropriate action based on the product requirements.
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
    /// use product_config::types::PropertyNameKind;
    /// use product_config::ProductConfigManager;
    /// use std::collections::HashMap;
    ///
    /// let config = ProductConfigManager::from_yaml_file("data/test_yamls/validate.yaml")
    ///     .unwrap();
    ///
    /// let mut user_data = HashMap::new();
    /// user_data.insert("ENV_INTEGER_PORT_MIN_MAX".to_string(), Some("12345".to_string()));
    /// user_data.insert("ENV_PROPERTY_STRING_MEMORY".to_string(), Some("1g".to_string()));
    ///
    /// let env_sh = config.get(
    ///     "0.5.0",
    ///     "role_1",
    ///     &PropertyNameKind::File("env.sh".to_string()),
    ///     user_data,
    /// );
    /// ```
    pub fn get(
        &self,
        version: &str,
        role: &str,
        kind: &PropertyNameKind,
        user_config: HashMap<String, Option<String>>,
    ) -> ValidationResult<BTreeMap<String, PropertyValidationResult>> {
        let product_version = semver_parse(version)?;

        // merge provided user properties with extracted property spec via role / kind and
        // dependencies to be validated later.
        let merged_properties = self
            .get_and_expand_properties(&product_version, role, kind, user_config)
            .unwrap();

        self.validate(&product_version, role, kind, merged_properties)
    }

    /// Merge the provided user config properties with the product configuration (loaded from YAML)
    /// depending on kind, role and version. The user configuration has the highest priority, followed
    /// by the recommended values from the product configuration. Finally, if none are available,
    /// the default values from the product configuration are used.
    /// This function also expands properties if they are required for the given role or if the user
    /// has requested so in the [user_config] parameter.
    ///
    ///
    /// # Arguments
    ///
    /// * `version` - the current product version
    /// * `role` - property role provided by the user
    /// * `kind` - property name kind provided by the user
    /// * `user_config` - map with property name and values (the explicit user config properties)
    pub(crate) fn get_and_expand_properties(
        &self,
        version: &Version,
        role: &str,
        kind: &PropertyNameKind,
        user_config: HashMap<String, Option<String>>,
    ) -> ValidationResult<BTreeMap<String, Option<String>>> {
        let mut merged_properties = BTreeMap::new();

        for property in &self.config.properties {
            let property_names = property.all_property_names();
            // If user provides a property that exists in the product config and fits the role and
            // version, we have to expand if needed.
            if util::hashmap_contains_any_key(&user_config, &property_names)
                && property.has_role(role)
                && property.is_version_supported(version)?
            {
                merged_properties.extend(expand_properties(property, version, role, kind)?);
            // If the user does not provide a property which is required in the product config,
            // and fits the role and version, we have to expand if needed.
            } else if property.has_role_required(role) && property.is_version_supported(version)? {
                if let Some((name, value)) = property.recommended_or_default(version, kind) {
                    merged_properties.insert(name, value);
                }
                merged_properties.extend(expand_properties(property, version, role, kind)?);
            }
        }

        // Add any unknown (not found in product config) properties provided by the user -> Overrides
        merged_properties.extend(user_config);

        // The user can provide "Meta" properties, that do not exists on their own and only expand
        // into other "valid" properties. Therefore it requires the "no_copy" field to indicate
        // that it should not end up in the final configuration.
        Ok(self.remove_no_copy_properties(version, role, kind, &merged_properties))
    }

    fn remove_no_copy_properties(
        &self,
        version: &Version,
        role: &str,
        kind: &PropertyNameKind,
        properties: &BTreeMap<String, Option<String>>,
    ) -> BTreeMap<String, Option<String>> {
        let mut result = BTreeMap::new();

        for (name, value) in properties {
            if let Some(prop) = self.find_property(&name, role, kind, version) {
                if prop.has_role_no_copy(role) {
                    continue;
                }
            }
            result.insert(name.clone(), value.clone());
        }

        result
    }

    /// Validates the given [merged_properties] by performing the following actions:
    /// * syntax checks on the values
    /// * mandatory checks (if a property is required for the given role and version)
    /// * comparison checks against the recommended and default values
    ///
    /// Properties that are not found in the product configuration are considered to be
    /// user "overrides".
    ///
    /// # Arguments
    /// * `version` - the current product version
    /// * `role` - property role provided by the user
    /// * `kind` - property name kind provided by the user
    /// * `merged_properties` - merged user and property spec (matching role, kind etc.)
    pub(crate) fn validate(
        &self,
        version: &Version,
        role: &str,
        kind: &PropertyNameKind,
        merged_properties: BTreeMap<String, Option<String>>,
    ) -> ValidationResult<BTreeMap<String, PropertyValidationResult>> {
        let mut result = BTreeMap::new();

        for (name, value) in merged_properties {
            let prop = self.find_property(&name, role, kind, version);

            match (prop, value) {
                (Some(property), Some(val)) => {
                    let check_datatype = validation::check_datatype(&property, &name, &val);
                    if let Err(err) = check_datatype {
                        result.insert(
                            name.to_string(),
                            PropertyValidationResult::Error(val.to_string(), err),
                        );
                        continue;
                    }

                    // TODO: what order? -> write tests for allowed_values and deprecated
                    if let Err(err) = check_allowed_values(&name, &val, &property.allowed_values) {
                        result.insert(
                            name.to_string(),
                            PropertyValidationResult::Error(val.to_string(), err),
                        );
                        continue;
                    }

                    if property.is_version_deprecated(version)? {
                        result.insert(
                            name.to_string(),
                            PropertyValidationResult::Warn(
                                val.to_string(),
                                error::Error::VersionDeprecated {
                                    property_name: name.to_string(),
                                    product_version: version.to_string(),
                                    // we would not reach here if deprecated_since is None
                                    // so we can just unwrap.
                                    deprecated_version: property.deprecated_since.unwrap(),
                                },
                            ),
                        );
                        continue;
                    }

                    // If we reach here the value is valid.
                    // Check if it was provided by recommended value?
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

                    // Check if it was provided by default value?
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
                // if required and not set -> error
                (Some(_property), None) => {
                    result.insert(
                        name.clone(),
                        PropertyValidationResult::Error(
                            name.to_string(),
                            error::Error::PropertyValueMissing {
                                property_name: name,
                            },
                        ),
                    );
                }
                // unknown
                (None, Some(val)) => {
                    result.insert(name, PropertyValidationResult::Unknown(val.to_string()));
                    continue;
                }
                _ => {}
            }
        }

        Ok(result)
    }

    fn find_property(
        &self,
        name: &str,
        role: &str,
        kind: &PropertyNameKind,
        version: &Version,
    ) -> Option<PropertySpec> {
        for property_anchor in &self.config.properties {
            if property_anchor.name_from_kind(kind) != Some(name.to_string()) {
                continue;
            }

            if !property_anchor.has_role(role) {
                continue;
            }

            if property_anchor.is_version_supported(version).is_err() {
                continue;
            }

            return Some(property_anchor.property.clone());
        }

        None
    }
}

#[cfg(test)]
mod tests {
    macro_rules! collection {
        // map-like
        ($($k:expr => $v:expr),* $(,)?) => {
            std::iter::Iterator::collect(std::array::IntoIter::new([$(($k, $v),)*]))
        };
        // set-like
        ($($v:expr),* $(,)?) => {
            std::iter::Iterator::collect(std::array::IntoIter::new([$($v,)*]))
        };
    }

    use std::collections::{BTreeMap, HashMap};

    use super::*;
    use crate::error::Error;
    use crate::types::PropertyNameKind;
    use crate::util::semver_parse;
    use crate::ProductConfigManager;
    use rstest::*;

    fn macro_to_hash_map(map: HashMap<String, Option<String>>) -> HashMap<String, Option<String>> {
        map
    }

    fn macro_to_btree_map(
        map: BTreeMap<String, Option<String>>,
    ) -> BTreeMap<String, Option<String>> {
        map
    }

    fn macro_to_get_result(
        map: BTreeMap<String, PropertyValidationResult>,
    ) -> BTreeMap<String, PropertyValidationResult> {
        map
    }

    #[rstest]
    #[case::expands_role_required_expandee_role_not_required(
        "0.5.0",
        &PropertyNameKind::File("env.sh".to_string()),
        "role_1",
        "data/test_yamls/expands_role_required_expandee_role_not_required.yaml",
        macro_to_hash_map(collection!{ "ENV_PASSWORD".to_string() => Some("secret".to_string()) }),
        macro_to_btree_map(collection!{
            "ENV_PASSWORD".to_string() => Some("secret".to_string()),
            "ENV_ENABLE_PASSWORD".to_string() => Some("true".to_string())
        }),
    )]
    #[case::expands_role_required_expandee_role_not_required_no_user_input(
        "0.5.0",
        &PropertyNameKind::File("env.sh".to_string()),
        "role_1",
        "data/test_yamls/expands_role_required_expandee_role_not_required.yaml",
        HashMap::new(),
        macro_to_btree_map(collection!{
            "ENV_PASSWORD".to_string() => None,
            "ENV_ENABLE_PASSWORD".to_string() => Some("true".to_string())
        }),
    )]
    #[case::expands_role_not_required_expandee_role_not_required_no_user_input(
        "0.5.0",
        &PropertyNameKind::File("env.sh".to_string()),
        "role_1",
        "data/test_yamls/expands_role_not_required_expandee_role_not_required.yaml",
        HashMap::new(),
        BTreeMap::new(),
    )]
    #[case::expands_role_not_required_expandee_role_required_no_user_input(
        "0.5.0",
        &PropertyNameKind::File("env.sh".to_string()),
        "role_1",
        "data/test_yamls/expands_role_not_required_expandee_role_required.yaml",
        HashMap::new(),
        macro_to_btree_map(collection!{
            "ENV_ENABLE_PASSWORD".to_string() => None,
        }),
    )]
    #[case::expands_role_not_required_expandee_role_required_with_user_input_1(
        "0.5.0",
        &PropertyNameKind::File("env.sh".to_string()),
        "role_1",
        "data/test_yamls/expands_role_not_required_expandee_role_required.yaml",
        macro_to_hash_map(collection!{
            "ENV_ENABLE_PASSWORD".to_string() => Some("true".to_string())
        }),
        macro_to_btree_map(collection!{
            "ENV_ENABLE_PASSWORD".to_string() => Some("true".to_string()),
        }),
    )]
    #[case::expands_role_not_required_expandee_role_required_with_user_input_2(
        "0.5.0",
        &PropertyNameKind::File("env.sh".to_string()),
        "role_1",
        "data/test_yamls/expands_role_not_required_expandee_role_required.yaml",
        macro_to_hash_map(collection!{
            "ENV_PASSWORD".to_string() => Some("secret".to_string())
        }),
        macro_to_btree_map(collection!{
            "ENV_PASSWORD".to_string() => Some("secret".to_string()),
            "ENV_ENABLE_PASSWORD".to_string() => Some("true".to_string()),
        }),
    )]
    #[case::expands_role_required_expandee_role_required_no_user_input(
        "0.5.0",
        &PropertyNameKind::File("env.sh".to_string()),
        "role_1",
        "data/test_yamls/expands_role_required_expandee_role_required.yaml",
        HashMap::new(),
        macro_to_btree_map(collection!{
            "ENV_PASSWORD".to_string() => None,
            "ENV_ENABLE_PASSWORD".to_string() => Some("true".to_string()),
        }),
    )]
    #[case::expands_role_required_expandee_role_required_with_user_input1(
        "0.5.0",
        &PropertyNameKind::File("env.sh".to_string()),
        "role_1",
        "data/test_yamls/expands_role_required_expandee_role_required.yaml",
        macro_to_hash_map(collection!{
            "ENV_PASSWORD".to_string() => Some("secret".to_string())
        }),
        macro_to_btree_map(collection!{
            "ENV_PASSWORD".to_string() => Some("secret".to_string()),
            "ENV_ENABLE_PASSWORD".to_string() => Some("true".to_string()),
        }),
    )]
    #[case::test_product_config_no_user_input(
        "0.5.0",
        &PropertyNameKind::File("env.sh".to_string()),
        "role_1",
        "data/test_yamls/test_product_config.yaml",
        HashMap::new(),
        macro_to_btree_map(collection!{
            "ENV_FLOAT".to_string() => Some("50.0".to_string()),
            "ENV_INTEGER_PORT_MIN_MAX".to_string() => Some("20000".to_string()),
            "ENV_PROPERTY_STRING_DEPRECATED".to_string() => None,
            "ENV_PASSWORD".to_string() => None,
            "ENV_ENABLE_PASSWORD".to_string() => Some("true".to_string()),
    }),
    )]
    #[case::expands_role_required_no_copy_no_user_input(
        "0.5.0",
        &PropertyNameKind::File("env.sh".to_string()),
        "role_1",
        "data/test_yamls/expands_role_required_no_copy.yaml",
        HashMap::new(),
        macro_to_btree_map(collection!{
            "ENV_SSL_CERTIFICATE_PATH".to_string() => Some("path/to/certificates".to_string()),
            "ENV_SSL_ENABLED".to_string() => Some("true".to_string()),
    }),
    )]
    #[case::expands_role_not_required_no_copy_no_user_input(
        "0.5.0",
        &PropertyNameKind::File("env.sh".to_string()),
        "role_1",
        "data/test_yamls/expands_role_not_required_no_copy.yaml",
        HashMap::new(),
        BTreeMap::new(),
    )]
    fn test_get_and_expand_properties(
        #[case] version: &str,
        #[case] kind: &PropertyNameKind,
        #[case] role: &str,
        #[case] path: &str,
        #[case] user_data: HashMap<String, Option<String>>,
        #[case] expected: BTreeMap<String, Option<String>>,
    ) {
        let product_version = semver_parse(version).unwrap();

        let manager = ProductConfigManager::from_yaml_file(path).unwrap();

        let result = manager
            .get_and_expand_properties(&product_version, role, kind, user_data)
            .unwrap();

        assert_eq!(result, expected);
    }

    #[rstest]
    #[case::get_no_user_input(
        &PropertyNameKind::File("env.sh".to_string()),
        "role_1",
        "data/test_yamls/validate.yaml",
        HashMap::new(),
        macro_to_get_result(collection!{
            "ENV_FLOAT".to_string() => PropertyValidationResult::RecommendedDefault("50.0".to_string()),
            "ENV_INTEGER_PORT_MIN_MAX".to_string() => PropertyValidationResult::RecommendedDefault("20000".to_string()),
            "ENV_ENABLE_PASSWORD".to_string() => PropertyValidationResult::Valid("true".to_string()),
            "ENV_PASSWORD".to_string() => PropertyValidationResult::Error("ENV_PASSWORD".to_string(), Error::PropertyValueMissing { property_name: "ENV_PASSWORD".to_string() }),
            "ENV_ENABLE_PASSWORD".to_string() => PropertyValidationResult::Valid("true".to_string()),
            "ENV_PROPERTY_STRING_DEPRECATED".to_string() => PropertyValidationResult::Warn("100mb".to_string(), Error::VersionDeprecated { property_name: "ENV_PROPERTY_STRING_DEPRECATED".to_string(), product_version: "0.5.0".to_string(), deprecated_version: "0.4.0".to_string() }),
        })
    )]
    #[case::get_valid_float(
        &PropertyNameKind::File("env.sh".to_string()),
        "role_1",
        "data/test_yamls/validate_float.yaml",
        macro_to_hash_map(collection!{
            "ENV_FLOAT".to_string() => Some("42.0".to_string())
        }),
        macro_to_get_result(collection!{
            "ENV_FLOAT".to_string() => PropertyValidationResult::Valid("42.0".to_string()),
        })
    )]
    #[case::get_recommended_float_no_user_input(
        &PropertyNameKind::File("env.sh".to_string()),
        "role_1",
        "data/test_yamls/validate_float.yaml",
        HashMap::new(),
        macro_to_get_result(collection!{
            "ENV_FLOAT".to_string() => PropertyValidationResult::RecommendedDefault("50.0".to_string()),
        })
    )]
    #[case::get_invalid_float_bad_user_value(
        &PropertyNameKind::File("env.sh".to_string()),
        "role_1",
        "data/test_yamls/validate_float.yaml",
        macro_to_hash_map(collection!{
            "ENV_FLOAT".to_string() => Some("CAFE".to_string())
        }),
        macro_to_get_result(collection!{
            "ENV_FLOAT".to_string() => PropertyValidationResult::Error("CAFE".to_string(), Error::DatatypeNotMatching { property_name: "ENV_FLOAT".to_string(), value: "CAFE".to_string(), datatype: "f64".to_string() }),
        })
    )]
    #[case::get_invalid_float_user_value_too_small(
        &PropertyNameKind::File("env.sh".to_string()),
        "role_1",
        "data/test_yamls/validate_float.yaml",
        macro_to_hash_map(collection!{
            "ENV_FLOAT".to_string() => Some("-1".to_string())
        }),
        macro_to_get_result(collection!{
            "ENV_FLOAT".to_string() => PropertyValidationResult::Error("-1".to_string(), Error::PropertyValueOutOfBounds { property_name: "ENV_FLOAT".to_string(), received: "-1".to_string(), expected: "0".to_string() }),
        })
    )]
    #[case::get_invalid_float_user_value_too_high(
        &PropertyNameKind::File("env.sh".to_string()),
        "role_1",
        "data/test_yamls/validate_float.yaml",
        macro_to_hash_map(collection!{
            "ENV_FLOAT".to_string() => Some("101".to_string())
        }),
        macro_to_get_result(collection!{
        "ENV_FLOAT".to_string() => PropertyValidationResult::Error("101".to_string(), Error::PropertyValueOutOfBounds { property_name: "ENV_FLOAT".to_string(), received: "101".to_string(), expected: "100".to_string() }),
        })
    )]
    #[case::get_invalid_ssl_certificate_path(
        &PropertyNameKind::Env,
        "role_1",
        "data/test_yamls/validate_directory.yaml",
        macro_to_hash_map(collection!{
            "ENV_SSL_CERTIFICATE_PATH".to_string() => Some("CAFE".to_string())
        }),
        macro_to_get_result(collection!{
            "ENV_SSL_CERTIFICATE_PATH".to_string() => PropertyValidationResult::Error("CAFE".to_string(), Error::DatatypeRegexNotMatching { property_name: "ENV_SSL_CERTIFICATE_PATH".to_string(), value: "CAFE".to_string() }),
        })
    )]
    #[case::get_valid_default_certificate_path_no_user_input(
        &PropertyNameKind::Env,
        "role_1",
        "data/test_yamls/validate_directory.yaml",
        HashMap::new(),
        macro_to_get_result(collection!{
            "ENV_SSL_CERTIFICATE_PATH".to_string() => PropertyValidationResult::Default("path/to/certificates".to_string()),
        })
    )]
    #[case::get_override_ssl_certificate_path(
        &PropertyNameKind::File("should_not_be_found_therefore_is_an_override".to_string()),
        "role_1",
        "data/test_yamls/validate_directory.yaml",
        macro_to_hash_map(collection!{
            "ENV_SSL_CERTIFICATE_PATH".to_string() => Some("/opt/stackable/zookeeper-operator/pki".to_string())
        }),
        macro_to_get_result(collection!{
            "ENV_SSL_CERTIFICATE_PATH".to_string() => PropertyValidationResult::Unknown("/opt/stackable/zookeeper-operator/pki".to_string()),
        })
    )]
    #[case::get_override_ssl_certificate_path(
        &PropertyNameKind::Env,
        "role_1",
        "data/test_yamls/validate_directory.yaml",
        macro_to_hash_map(collection!{
            "ENV_SSL_CERTIFICATE_PATH".to_string() => Some("/opt/stackable/zookeeper-operator/pki".to_string())
        }),
        macro_to_get_result(collection!{
            "ENV_SSL_CERTIFICATE_PATH".to_string() => PropertyValidationResult::Valid("/opt/stackable/zookeeper-operator/pki".to_string()),
        })
    )]
    #[case::get_recommended_port_no_user_input(
        &PropertyNameKind::Env,
        "role_1",
        "data/test_yamls/validate_port.yaml",
        HashMap::new(),
        macro_to_get_result(collection!{
            "ENV_INTEGER_PORT_MIN_MAX".to_string() => PropertyValidationResult::RecommendedDefault("20000".to_string()),
        })
    )]
    #[case::get_port_user_value_too_small(
        &PropertyNameKind::Env,
        "role_1",
        "data/test_yamls/validate_port.yaml",
        macro_to_hash_map(collection!{
            "ENV_INTEGER_PORT_MIN_MAX".to_string() => Some("42".to_string())
        }),
        macro_to_get_result(collection!{
            "ENV_INTEGER_PORT_MIN_MAX".to_string() => PropertyValidationResult::Error("42".to_string(), Error::PropertyValueOutOfBounds { property_name: "ENV_INTEGER_PORT_MIN_MAX".to_string(), received: "42".to_string(), expected: "1024".to_string() })
        })
    )]
    #[case::get_port_user_value_too_high(
        &PropertyNameKind::Env,
        "role_1",
        "data/test_yamls/validate_port.yaml",
        macro_to_hash_map(collection!{
            "ENV_INTEGER_PORT_MIN_MAX".to_string() => Some("65536".to_string())
        }),
        macro_to_get_result(collection!{
        "ENV_INTEGER_PORT_MIN_MAX".to_string() => PropertyValidationResult::Error("65536".to_string(), Error::PropertyValueOutOfBounds { property_name: "ENV_INTEGER_PORT_MIN_MAX".to_string(), received: "65536".to_string(), expected: "65535".to_string() })
        })
    )]
    #[case::get_port_user_value_invalid(
        &PropertyNameKind::Env,
        "role_1",
        "data/test_yamls/validate_port.yaml",
        macro_to_hash_map(collection!{
            "ENV_INTEGER_PORT_MIN_MAX".to_string() => Some("invalid".to_string())
        }),
        macro_to_get_result(collection!{
            "ENV_INTEGER_PORT_MIN_MAX".to_string() => PropertyValidationResult::Error("invalid".to_string(), Error::DatatypeNotMatching { property_name: "ENV_INTEGER_PORT_MIN_MAX".to_string(), value: "invalid".to_string(), datatype: "i64".to_string() })
        })
    )]
    #[case::get_port_user_value_valid(
        &PropertyNameKind::Env,
        "role_1",
        "data/test_yamls/validate_port.yaml",
        macro_to_hash_map(collection!{
            "ENV_INTEGER_PORT_MIN_MAX".to_string() => Some("1024".to_string()),
        }),
        macro_to_get_result(collection!{
            "ENV_INTEGER_PORT_MIN_MAX".to_string() => PropertyValidationResult::Valid("1024".to_string()),
        })
    )]
    fn test_get(
        #[case] kind: &PropertyNameKind,
        #[case] role: &str,
        #[case] path: &str,
        #[case] user_data: HashMap<String, Option<String>>,
        #[case] expected: BTreeMap<String, PropertyValidationResult>,
    ) -> ValidationResult<()> {
        let manager = ProductConfigManager::from_yaml_file(path).unwrap();

        let result = manager.get("0.5.0", role, kind, user_data)?;

        assert_eq!(result, expected);

        Ok(())
    }
}
