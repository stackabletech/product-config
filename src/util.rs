use crate::error::Error;
use crate::types::{
    PropertyDependency, PropertyName, PropertyNameKind, PropertySpec, PropertyValueSpec,
};
use crate::validation::ValidationResult;
use semver::Version;
use std::collections::HashMap;

/// Automatically retrieve and validate config properties from the property spec that:
/// - match the provided kind (e.g. Conf(my.config))
/// - match the role and are required
/// - are available for the current product version
/// - have dependencies dependency with a provided value or property that has recommended value
///
/// # Arguments
///
/// * `property_spec` - map with PropertyName as key and the corresponding PropertySpec as value
/// * `kind` - config kind provided by the user -> relate to config_option.option_name.kind
/// * `role` - the role required / used for the config options
/// * `product_version` - the provided product version
///
pub fn get_matching_config_options(
    property_spec: &HashMap<PropertyName, PropertySpec>,
    kind: &PropertyNameKind,
    role: Option<&str>,
    product_version: &Version,
) -> ValidationResult<HashMap<String, String>> {
    let mut config_file_options = HashMap::new();

    for (property_name, spec) in property_spec {
        // ignore this option if kind does not match
        // TODO: improve performance by sorting config options per kind
        if &property_name.kind != kind {
            continue;
        }

        // Ignore this configuration option if it is only available (specified via `as_of_version`)
        // in later versions than the one we're checking against.
        if Version::parse(spec.as_of_version.as_str())? > *product_version {
            continue;
        }

        // ignore completely if role is None
        // ignore this option if role does not match or is not required
        if let Some(config_option_roles) = &spec.roles {
            for config_option_role in config_option_roles {
                // role found?
                if Some(config_option_role.name.as_str()) == role && config_option_role.required {
                    // check for recommended value and matching version
                    if let Some(recommended) = &spec.recommended_values {
                        let property_value = get_property_value_for_version(
                            property_name,
                            recommended,
                            product_version,
                        )?;

                        config_file_options
                            .insert(property_name.name.clone(), property_value.value);

                        // check for dependencies
                        if let Some(config_option_dependencies) = &spec.depends_on {
                            // dependency found
                            let dependencies = get_config_dependencies_and_values(
                                property_spec,
                                product_version,
                                property_name,
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

/// Collect all dependencies that are required based on user properties
///
/// # Arguments
///
/// * `property_spec` - map with PropertyName as key and the corresponding PropertySpec as value
/// * `user_config` - map with the user config names and according values
/// * `version` - the provided product version
/// * `kind` - config kind provided by the user -> relate to config_option.option_name.kind
///
pub fn get_matching_dependencies(
    property_spec: &HashMap<PropertyName, PropertySpec>,
    user_config: &HashMap<String, String>,
    version: &Version,
    kind: &PropertyNameKind,
) -> ValidationResult<HashMap<String, String>> {
    let mut user_dependencies = HashMap::new();
    for name in user_config.keys() {
        let option_name = PropertyName {
            name: name.clone(),
            kind: kind.clone(),
        };

        if let Some(option) = property_spec.get(&option_name) {
            if let Some(dependencies) = &option.depends_on {
                user_dependencies.extend(get_config_dependencies_and_values(
                    property_spec,
                    version,
                    &option_name,
                    dependencies,
                )?);
            }
        }
    }

    Ok(user_dependencies)
}

/// Collect all dependencies for required properties and extract a value from either the dependency
/// itself, or if not available the recommended value of the property dependency
///
/// # Arguments
///
/// * `property_spec` - map with PropertyName as key and the corresponding PropertySpec as value
/// * `product_version` - the provided product version
/// * `property_name` - name of the property
/// * `property_dependencies` - the dependencies of the property to check
///
fn get_config_dependencies_and_values(
    property_spec: &HashMap<PropertyName, PropertySpec>,
    product_version: &Version,
    property_name: &PropertyName,
    property_dependencies: &[PropertyDependency],
) -> ValidationResult<HashMap<String, String>> {
    let mut dependencies = HashMap::new();
    for option_dependency in property_dependencies {
        for dependency_option_name in &option_dependency.property_names {
            // the dependency should not differ in the kind
            if property_name.kind == dependency_option_name.kind {
                // if the dependency has a proposed value we are done
                if let Some(dependency_value) = &option_dependency.value {
                    dependencies.insert(
                        dependency_option_name.name.clone(),
                        dependency_value.clone(),
                    );
                }
                // we check the dependency for a recommended value
                if let Some(dependency_config_option) = property_spec.get(dependency_option_name) {
                    if let Some(recommended) = &dependency_config_option.recommended_values {
                        let recommended_value = get_property_value_for_version(
                            property_name,
                            recommended,
                            product_version,
                        )?;
                        dependencies
                            .insert(dependency_option_name.name.clone(), recommended_value.value);
                    }
                }
            }
        }
    }

    Ok(dependencies)
}

/// Extract the provided value from recommended / default values that matches the product version.
/// Check if there exists a recommended / default value that has a range (if provided) with
/// from_version and to_version that includes the product version. E.g. if from_version is 1.0.0
/// and to_version is 1.9.99, we have a value for product version 1.5.0 but not 2.0.0.
///
/// # Arguments
///
/// * `property_name` - name of the property
/// * `property_values` - list of property values and their respective versions
/// * `product_version` - the product version
///
pub fn get_property_value_for_version(
    property_name: &PropertyName,
    property_values: &[PropertyValueSpec],
    product_version: &Version,
) -> ValidationResult<PropertyValueSpec> {
    for value in property_values {
        if let Some(from) = &value.from_version {
            let from_version = Version::parse(from)?;

            if from_version > *product_version {
                continue;
            }
        }

        if let Some(to) = &value.to_version {
            let to_version = Version::parse(to)?;

            if to_version < *product_version {
                continue;
            }
        }

        return Ok(value.clone());
    }

    Err(Error::PropertySpecValueMissingForVersion {
        property_name: property_name.clone(),
        property_values: Vec::from(property_values),
        version: product_version.to_string(),
    })
}
