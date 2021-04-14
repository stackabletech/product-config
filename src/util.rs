use crate::error::Error;
use crate::types::{ConfigOption, Dependency, OptionKind, OptionName, OptionValue};
use crate::validation::ConfigValidationResult;
use semver::Version;
use std::collections::HashMap;

/// Collect all config_options that match:
/// - provided kind
/// - the controller version
/// - provided role + required == true (if role is None is wildcard for add all)
/// - a dependency with a provided value or option that has recommended value
///
/// # Arguments
///
/// * `config_options` - map with OptionName as key and the corresponding ConfigOption as value
/// * `kind` - config kind provided by the user -> relate to config_option.option_name.kind
/// * `role` - the role required / used for the config options
/// * `version` - the provided product version
///
pub fn get_matching_config_options(
    config_options: &HashMap<OptionName, ConfigOption>,
    kind: &OptionKind,
    role: Option<&str>,
    version: &str,
) -> ConfigValidationResult<HashMap<String, String>> {
    let mut config_file_options = HashMap::new();

    for (option_name, config_option) in config_options {
        // ignore this option if kind does not match
        if &option_name.kind != kind {
            continue;
        }

        // ignore if version higher than controller version
        if Version::parse(config_option.as_of_version.as_str())? > Version::parse(version)? {
            continue;
        }

        // ignore completely if role is None
        // ignore this option if role does not match or is not required
        if let Some(config_option_roles) = &config_option.roles {
            for config_option_role in config_option_roles {
                // role found?
                if Some(config_option_role.name.as_str()) == role && config_option_role.required {
                    // check for recommended value and matching version
                    if let Some(recommended) = &config_option.recommended_values {
                        let option_value =
                            get_option_value_for_version(option_name, recommended, version)?;

                        config_file_options.insert(option_name.name.clone(), option_value.value);

                        // check for dependencies
                        if let Some(config_option_dependencies) = &config_option.depends_on {
                            // dependency found
                            let dependencies = get_config_dependencies_and_values(
                                config_options,
                                version,
                                option_name,
                                &config_option_dependencies,
                            )?;

                            config_file_options.extend(dependencies);
                        }
                    }
                }
            }
        }
    }

    Ok(config_file_options)
}

/// Collect all dependencies that are required because of user config options
///
/// # Arguments
///
/// * `config_options` - map with OptionName as key and the corresponding ConfigOption as value
/// * `user_config` - map with the user config names and according values
/// * `version` - the provided controller version
/// * `kind` - config kind provided by the user -> relate to config_option.option_name.kind
///
pub fn get_matching_dependencies(
    config_options: &HashMap<OptionName, ConfigOption>,
    user_config: &HashMap<String, String>,
    version: &str,
    kind: &OptionKind,
) -> ConfigValidationResult<HashMap<String, String>> {
    let mut user_dependencies = HashMap::new();
    for name in user_config.keys() {
        let option_name = OptionName {
            name: name.clone(),
            kind: kind.clone(),
        };

        if let Some(option) = config_options.get(&option_name) {
            if let Some(dependencies) = &option.depends_on {
                user_dependencies.extend(get_config_dependencies_and_values(
                    config_options,
                    version,
                    &option_name,
                    dependencies,
                )?);
            }
        }
    }

    Ok(user_dependencies)
}

/// Collect all dependencies for required config options and extract a value from either
/// the dependency itself, or if not available the recommended value of the dependant
/// dependency config option
///
/// # Arguments
///
/// * `config_options` - map with OptionName as key and the corresponding ConfigOption as value
/// * `version` - the provided product version
/// * `option_name` - name of the config option
/// * `option_dependencies` - the dependencies of the option to check
///
fn get_config_dependencies_and_values(
    config_options: &HashMap<OptionName, ConfigOption>,
    version: &str,
    option_name: &OptionName,
    option_dependencies: &[Dependency],
) -> ConfigValidationResult<HashMap<String, String>> {
    let mut dependencies = HashMap::new();
    for option_dependency in option_dependencies {
        for dependency_option_name in &option_dependency.option_names {
            // the dependency should not differ in the kind
            if option_name.kind == dependency_option_name.kind {
                // if the dependency has a proposed value we are done
                if let Some(dependency_value) = &option_dependency.value {
                    dependencies.insert(
                        dependency_option_name.name.clone(),
                        dependency_value.clone(),
                    );
                }
                // we check the dependency for a recommended value
                if let Some(dependency_config_option) = config_options.get(dependency_option_name) {
                    if let Some(recommended) = &dependency_config_option.recommended_values {
                        let recommended_value =
                            get_option_value_for_version(option_name, recommended, version)?;
                        dependencies
                            .insert(dependency_option_name.name.clone(), recommended_value.value);
                    }
                }
            }
        }
    }

    Ok(dependencies)
}

/// Get the correct value from recommended / default values depending on the version
///
/// # Arguments
///
/// * `option_name` - name of the config option
/// * `option_values` - list of option values and their respective versions
/// * `version` - product / controller version
///
pub fn get_option_value_for_version(
    option_name: &OptionName,
    option_values: &[OptionValue],
    version: &str,
) -> ConfigValidationResult<OptionValue> {
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

    Err(Error::ConfigValueMissingForVersion {
        option_name: option_name.clone(),
        option_values: Vec::from(option_values),
        version: version.to_string(),
    })
}
