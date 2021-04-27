use crate::types::{PropertyDependency, PropertyValueSpec};
use crate::PropertyName;

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

    #[error("[{property_name}]: current product version is '{product_version}' -> property not supported; available from version '{required_version}'")]
    VersionNotSupported {
        property_name: PropertyName,
        product_version: String,
        required_version: String,
    },

    #[error("[{property_name}]: current product version is '{product_version}' -> property deprecated since version '{deprecated_version}'")]
    VersionDeprecated {
        property_name: PropertyName,
        product_version: String,
        deprecated_version: String,
    },

    #[error("Required config spec property not found: '{name}'")]
    ConfigSpecPropertiesNotFound { name: String },

    #[error("No config property found that matches '{property_name}'")]
    PropertyNotFound { property_name: PropertyName },

    #[error("No roles in '{name}' match the provided role: '{role}'")]
    PropertySpecRoleNotFound { name: PropertyName, role: String },

    #[error("No property roles provided for '{name}' ")]
    PropertySpecRoleNotProvided { name: PropertyName },

    #[error("No role was provided by user for '{name}' ")]
    PropertySpecRoleNotProvidedByUser { name: PropertyName },

    #[error("[{0}]: provided value '{received}' violates min/max bound '{expected}'")]
    PropertyValueOutOfBounds {
        property_name: PropertyName,
        received: String,
        expected: String,
    },

    #[error("[{property_name}]: provided config value missing")]
    PropertyValueMissing { property_name: PropertyName },

    #[error("[{property_name}]: provided property value(s) missing for version '{version}'. Got: {property_values:?}")]
    PropertySpecValueMissingForVersion {
        property_name: PropertyName,
        property_values: Vec<PropertyValueSpec>,
        version: String,
    },

    #[error("[{property_name}]: value '{value}' not in allowed values: {allowed_values:?}")]
    PropertyValueNotInAllowedValues {
        property_name: PropertyName,
        value: String,
        allowed_values: Vec<String>,
    },

    #[error("[{property_name}]: value '{value}' not of specified type: '{datatype}'")]
    DatatypeNotMatching {
        property_name: PropertyName,
        value: String,
        datatype: String,
    },

    #[error("[{property_name}]: value '{value}' does not match regex")]
    DatatypeRegexNotMatching {
        property_name: PropertyName,
        value: String,
    },

    #[error("Empty regex pattern for unit '{unit}'")]
    EmptyRegexPattern { unit: String },

    #[error("Invalid regex pattern for unit '{unit}': '{regex}'")]
    InvalidRegexPattern { unit: String, regex: String },

    #[error("[{property_name}]: unit not provided")]
    UnitNotProvided { property_name: PropertyName },

    #[error("[{property_name}]: unit '{unit}' not found in settings")]
    UnitSettingNotFound {
        property_name: PropertyName,
        unit: String,
    },

    #[error("[{property_name}]: required dependency not provided: '{dependency:?}'")]
    PropertyDependencyMissing {
        property_name: PropertyName,
        dependency: Vec<PropertyName>,
    },

    #[error(
        "[{property_name}]: dependency '{dependency}' requires no values, but was set to '{user_value}'"
    )]
    PropertyDependencyUserValueNotRequired {
        property_name: PropertyName,
        dependency: String,
        user_value: String,
    },

    #[error(
        "[{property_name}]: dependency '{dependency}' requires value '{required_value}' to be set"
    )]
    PropertyDependencyUserValueMissing {
        property_name: PropertyName,
        dependency: String,
        required_value: String,
    },

    #[error(
        "[{property_name}]: provided value '{user_value} does not match required value '{required_value}' for dependency '{dependency}'"
    )]
    PropertyDependencyValueInvalid {
        property_name: PropertyName,
        dependency: String,
        user_value: String,
        required_value: String,
    },

    #[error("[{property_name}]: no provided or recommended values in dependency '{dependency:?}'")]
    PropertyDependencyValueMissing {
        property_name: PropertyName,
        dependency: PropertyDependency,
    },
}
