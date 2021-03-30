use crate::types::OptionName;

/// warning definitions
#[derive(thiserror::Error, Clone, Debug, PartialOrd, PartialEq)]
pub enum Warning {
    #[error("[{name}]: provided config value missing")]
    ConfigOptionValueMissing { name: OptionName },

    #[error("No config role provided for '{name}'.")]
    ConfigOptionRoleNotProvided { name: OptionName },

    #[error("No roles in '{name}' match the provided role: '{role}'")]
    ConfigOptionRoleNotFound { name: OptionName, role: String },
}
