use crate::error::Error;
use crate::types::{ConfigOption, OptionName, OptionValue};
use semver::Version;
use std::collections::HashMap;

pub fn filter_config_options(
    config_options: &HashMap<OptionName, ConfigOption>,
    config_file: &str,
    config_role: Option<&str>,
    version: &str,
) -> Result<HashMap<String, Option<String>>, Error> {
    let mut config_file_options = HashMap::new();

    for (option_name, config_option) in config_options {
        // config file matches?
        if option_name.config_file != config_file {
            continue;
        }

        // role exists and matches?
        // TODO: Right now not providing a role is ignored. Throw error?
        if let Some(role) = config_role {
            if !option_role_matches(&config_option, role) {
                continue;
            }
        }

        // TODO: What if no recommended or default value provided?
        if let Some(recommended) = &config_option.recommended_values {
            let option_value = filter_option_value_for_version(recommended, option_name, version)?;
            config_file_options.insert(option_name.name.clone(), Some(option_value.value));
        } else if let Some(default) = &config_option.default_values {
            let option_value = filter_option_value_for_version(default, option_name, version)?;
            config_file_options.insert(option_name.name.clone(), Some(option_value.value));
        }
    }

    Ok(config_file_options)
}

pub fn option_role_matches(config_option: &ConfigOption, config_role: &str) -> bool {
    let mut role_match = false;
    if let Some(roles) = &config_option.roles {
        for role in roles {
            if role.required && role.name == config_role {
                role_match = true;
                break;
            }
        }
    }
    role_match
}

pub fn filter_option_value_for_version(
    option_values: &[OptionValue],
    option_name: &OptionName,
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
