use crate::error::Error;
use crate::types::{ConfigOption, OptionKind, OptionName, OptionValue};
use semver::Version;
use std::collections::HashMap;

/// Check if the final used value corresponds to e.g. recommended or default values.
///
/// # Arguments
///
/// * `config_options` - map with OptionName as key and the corresponding ConfigOption as value
/// * `kind` - config kind provided by the user -> relate to config_option.option_name.kind
/// * `role` - the role required / used for the config options
/// * `version` - the provided product version
///
pub fn filter_config_options(
    config_options: &HashMap<OptionName, ConfigOption>,
    kind: &OptionKind,
    role: Option<&str>,
    version: &str,
) -> Result<HashMap<String, String>, Error> {
    let mut config_file_options = HashMap::new();

    for (option_name, config_option) in config_options {
        // config file matches?
        if option_name.kind.get_file_name() != kind.get_file_name() {
            continue;
        }

        // role exists and matches?
        // TODO: Right now, not providing a role is ignored. Throw error?
        if let Some(role) = role {
            if !option_role_matches(&config_option, role) {
                continue;
            }
        }

        // TODO: What if no recommended or default value provided?
        if let Some(recommended) = &config_option.recommended_values {
            let option_value = filter_option_value_for_version(option_name, recommended, version)?;
            config_file_options.insert(option_name.name.clone(), option_value.value);
        } else if let Some(default) = &config_option.default_values {
            let option_value = filter_option_value_for_version(option_name, default, version)?;
            config_file_options.insert(option_name.name.clone(), option_value.value);
        }
    }

    Ok(config_file_options)
}

/// Check if the provided config role matches the "required" config option role
///
/// # Arguments
///
/// * `config_options` - map with (defined) config option names and the respective config_option
/// * `role` - the role required / used for the config options
///
pub fn option_role_matches(config_option: &ConfigOption, user_role: &str) -> bool {
    let mut role_match = false;
    if let Some(roles) = &config_option.roles {
        for role in roles {
            if role.required && role.name == user_role {
                role_match = true;
                break;
            }
        }
    }
    role_match
}

/// Get the correct value from recommended / default values depending on the version
///
/// # Arguments
///
/// * `option_name` - name of the config option (config property or environmental variable)
/// * `option_values` - list of option values and their respective versions
/// * `version` - product / controller version
///
pub fn filter_option_value_for_version(
    option_name: &OptionName,
    option_values: &[OptionValue],
    version: &str,
) -> Result<OptionValue, Error> {
    let product_version = Version::parse(version)?;

    for value in option_values {
        if let Some(from) = &value.from_version {
            let from_version = Version::parse(from)?;

            if from_version > product_version {
                continue;
            }
        }

        if let Some(to) = &value.to_version {
            let to_version = Version::parse(to)?;

            if to_version < product_version {
                continue;
            }
        }

        return Ok(value.clone());
    }

    Err(Error::ConfigValueMissing {
        option_name: option_name.clone(),
    })
}
