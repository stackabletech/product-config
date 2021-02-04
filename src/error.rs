/// error definitions
#[derive(PartialEq, thiserror::Error, Debug)]
pub enum Error {
    // reader
    #[error("file not found: {file_name}")]
    FileNotFound { file_name: String },

    #[error("could not parse file: {file_name}: {reason}")]
    FileNotParsable { file_name: String, reason: String },

    // version
    #[error("invalid sem version: {source}")]
    InvalidVersion {
        #[from]
        source: semver::SemVerError,
    },

    #[error("[{option_name}]: current controller version is [{product_version}] -> option not supported; available from version [{required_version}]")]
    VersionNotSupported {
        option_name: String,
        product_version: String,
        required_version: String,
    },

    #[error("[{option_name}]: current controller version is [{product_version}] -> option deprecated since version [{deprecated_version}]")]
    VersionDeprecated {
        option_name: String,
        product_version: String,
        deprecated_version: String,
    },

    // config
    #[error("required config setting not found: '{name}'")]
    ConfigSettingNotFound { name: String },

    #[error("no config option found that matches '{option_name}'")]
    ConfigOptionNotFound { option_name: String },

    #[error("[{0}]: provided value '{received}' violates min/max bound '{expected}'")]
    ConfigValueOutOfBounds {
        option_name: String,
        received: String,
        expected: String,
    },

    #[error("[{option_name}]: provided config value missing")]
    ConfigValueMissing { option_name: String },

    #[error("[{option_name}]: value '{value}' not in allowed values: {allowed_values:?}")]
    ConfigValueNotInAllowedValues {
        option_name: String,
        value: String,
        allowed_values: Vec<String>,
    },

    // parsing
    #[error("[{option_name}]: value '{value}' not of specified type: '{datatype}'")]
    DatatypeNotMatching {
        option_name: String,
        value: String,
        datatype: String,
    },

    #[error("[{option_name}]: value '{value}' does not match regex")]
    DatatypeRegexNotMatching { option_name: String, value: String },

    #[error("empty regex pattern for unit '{unit}'")]
    EmptyRegexPattern { unit: String },

    #[error("invalid regex pattern for unit '{unit}': {regex}")]
    InvalidRegexPattern { unit: String, regex: String },

    #[error("[{option_name}]: unit not provided")]
    UnitNotProvided { option_name: String },

    #[error("[{option_name}]: unit '{unit}' not found in settings")]
    UnitSettingNotFound { option_name: String, unit: String },
}
