use std::path::PathBuf;

use snafu::Snafu;

use crate::types::PropertyValueSpec;
use crate::PropertyName;

#[derive(Clone, Debug, PartialOrd, PartialEq, Snafu)]
pub enum Error {
    #[snafu(display("file not found: {}", file_name.display()))]
    FileNotFound { file_name: PathBuf },

    #[snafu(display("could not parse yaml file - {}: {reason}", file.display()))]
    YamlFileNotParsable { file: PathBuf, reason: String },

    #[snafu(display("could not parse yaml - {content}: {reason}"))]
    YamlNotParsable { content: String, reason: String },

    #[snafu(display("failed to parse '{version}' as SemVer version: {reason}"))]
    InvalidVersion { reason: String, version: String },

    #[snafu(display("[{property_name}]: current product version is '{product_version}' -> property not supported; available from version '{required_version}'"))]
    VersionNotSupported {
        property_name: PropertyName,
        product_version: String,
        required_version: String,
    },

    #[snafu(display("[{property_name}]: current product version is '{product_version}' -> property deprecated since version '{deprecated_version}'"))]
    VersionDeprecated {
        property_name: String,
        product_version: String,
        deprecated_version: String,
    },

    #[snafu(display("required config spec property not found: '{name}'"))]
    ConfigSpecPropertiesNotFound { name: String },

    #[snafu(display("no config property found that matches '{property_name}'"))]
    PropertyNotFound { property_name: PropertyName },

    #[snafu(display("no roles in '{name}' match the provided role: '{role}'"))]
    PropertySpecRoleNotFound { name: PropertyName, role: String },

    #[snafu(display("no property roles provided for '{name}' "))]
    PropertySpecRoleNotProvided { name: PropertyName },

    #[snafu(display("no role was provided by user for '{name}' "))]
    PropertySpecRoleNotProvidedByUser { name: PropertyName },

    #[snafu(display(
        "[{property_name}]: provided value '{received}' violates min/max bound '{expected}'"
    ))]
    PropertyValueOutOfBounds {
        property_name: String,
        received: String,
        expected: String,
    },

    #[snafu(display("[{property_name}]: config value missing for required property"))]
    PropertyValueMissing { property_name: String },

    #[snafu(display("[{property_name}]: provided property value(s) missing for version '{version}'. Got: {property_values:?}"))]
    PropertySpecValueMissingForVersion {
        property_name: PropertyName,
        property_values: Vec<PropertyValueSpec>,
        version: String,
    },

    #[snafu(display(
        "[{property_name}]: value '{value}' not in allowed values: {allowed_values:?}"
    ))]
    PropertyValueNotInAllowedValues {
        property_name: String,
        value: String,
        allowed_values: Vec<String>,
    },

    #[snafu(display("[{property_name}]: value '{value}' not of specified type: '{datatype}'"))]
    DatatypeNotMatching {
        property_name: String,
        value: String,
        datatype: String,
    },

    #[snafu(display("[{property_name}]: value '{value}' does not match regex"))]
    DatatypeRegexNotMatching {
        property_name: String,
        value: String,
    },

    #[snafu(display("empty regex pattern for unit '{unit}'"))]
    EmptyRegexPattern { unit: String },

    #[snafu(display("invalid regex pattern for unit '{unit}': '{regex}'"))]
    InvalidRegexPattern { unit: String, regex: String },

    #[snafu(display("the regex for unit '{unit}' ('{regex}') could not be evaluated on property '{property_name}' (value: '{value}'): {reason}."))]
    RegexNotEvaluable {
        property_name: String,
        unit: String,
        regex: String,
        value: String,
        reason: String,
    },

    #[snafu(display("[{property_name}]: unit not provided"))]
    UnitNotProvided { property_name: PropertyName },

    #[snafu(display("[{property_name}]: unit '{unit}' not found in settings"))]
    UnitSettingNotFound {
        property_name: PropertyName,
        unit: String,
    },
}
