use crate::types::{Dependency, OptionValue};
use crate::ConfigName;

#[derive(thiserror::Error, Clone, Debug, PartialOrd, PartialEq)]
pub enum Error {
    #[error("file not found: {file_name}")]
    FileNotFound { file_name: String },

    #[error("could not parse file: {file_name}: {reason}")]
    FileNotParsable { file_name: String, reason: String },

    #[error("Invalid sem version: {source}")]
    InvalidVersion {
        #[from]
        source: semver::SemVerError,
    },

    #[error("[{option_name}]: current product version is '{product_version}' -> option not supported; available from version '{required_version}'")]
    VersionNotSupported {
        option_name: ConfigName,
        product_version: String,
        required_version: String,
    },

    #[error("[{option_name}]: current product version is '{product_version}' -> option deprecated since version '{deprecated_version}'")]
    VersionDeprecated {
        option_name: ConfigName,
        product_version: String,
        deprecated_version: String,
    },

    #[error("Required config setting not found: '{name}'")]
    ConfigSettingNotFound { name: String },

    #[error("No config option found that matches '{option_name}'")]
    ConfigOptionNotFound { option_name: ConfigName },

    #[error("No roles in '{name}' match the provided role: '{role}'")]
    ConfigOptionRoleNotFound { name: ConfigName, role: String },

    #[error("No config option roles provided for '{name}' ")]
    ConfigOptionRoleNotProvided { name: ConfigName },

    #[error("No role was provided by user for '{name}' ")]
    ConfigOptionRoleNotProvidedByUser { name: ConfigName },

    #[error("[{0}]: provided value '{received}' violates min/max bound '{expected}'")]
    ConfigValueOutOfBounds {
        option_name: ConfigName,
        received: String,
        expected: String,
    },

    #[error("[{option_name}]: provided config value missing")]
    ConfigValueMissing { option_name: ConfigName },

    #[error("[{option_name}]: provided config value(s) missing for version '{version}'. Got: {option_values:?}")]
    ConfigValueMissingForVersion {
        option_name: ConfigName,
        option_values: Vec<OptionValue>,
        version: String,
    },

    #[error("[{option_name}]: value '{value}' not in allowed values: {allowed_values:?}")]
    ConfigValueNotInAllowedValues {
        option_name: ConfigName,
        value: String,
        allowed_values: Vec<String>,
    },

    #[error("[{option_name}]: value '{value}' not of specified type: '{datatype}'")]
    DatatypeNotMatching {
        option_name: ConfigName,
        value: String,
        datatype: String,
    },

    #[error("[{option_name}]: value '{value}' does not match regex")]
    DatatypeRegexNotMatching {
        option_name: ConfigName,
        value: String,
    },

    #[error("Empty regex pattern for unit '{unit}'")]
    EmptyRegexPattern { unit: String },

    #[error("Invalid regex pattern for unit '{unit}': '{regex}'")]
    InvalidRegexPattern { unit: String, regex: String },

    #[error("[{option_name}]: unit not provided")]
    UnitNotProvided { option_name: ConfigName },

    #[error("[{option_name}]: unit '{unit}' not found in settings")]
    UnitSettingNotFound {
        option_name: ConfigName,
        unit: String,
    },

    #[error("[{option_name}]: required dependency not provided: '{dependency:?}'")]
    ConfigDependencyMissing {
        option_name: ConfigName,
        dependency: Vec<ConfigName>,
    },

    #[error(
        "[{option_name}]: dependency '{dependency}' requires no values, but was set to '{user_value}'"
    )]
    ConfigDependencyUserValueNotRequired {
        option_name: ConfigName,
        dependency: String,
        user_value: String,
    },

    #[error(
        "[{option_name}]: dependency '{dependency}' requires value '{required_value}' to be set"
    )]
    ConfigDependencyUserValueMissing {
        option_name: ConfigName,
        dependency: String,
        required_value: String,
    },

    #[error(
        "[{option_name}]: provided value '{user_value} does not match required value '{required_value}' for dependency '{dependency}'"
    )]
    ConfigDependencyValueInvalid {
        option_name: ConfigName,
        dependency: String,
        user_value: String,
        required_value: String,
    },

    #[error("[{option_name}]: no provided or recommended values in dependency '{dependency:?}'")]
    ConfigDependencyValueMissing {
        option_name: ConfigName,
        dependency: Dependency,
    },
}
