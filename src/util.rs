use crate::error::Error;
use crate::types::{
    PropertyDependency, PropertyName, PropertyNameKind, PropertySpec, PropertyValueSpec,
};
use crate::validation::ValidationResult;
use semver::Version;
use std::collections::HashMap;

/// Automatically retrieve and validate config properties from the property spec that:
/// - match the provided kind (e.g. File(my.config))
/// - match the role and are required
/// - are available for the current product version
/// - have dependencies with a provided value or property that has a recommended value
///
/// # Arguments
///
/// * `property_spec` - map with property name as key and the corresponding property spec as value
/// * `kind` - property name kind provided by the user
/// * `role` - the role required / used for the property
/// * `product_version` - the provided product version
///
pub(crate) fn get_matching_properties(
    property_spec: &HashMap<PropertyName, PropertySpec>,
    kind: &PropertyNameKind,
    role: Option<&str>,
    product_version: &Version,
) -> ValidationResult<HashMap<String, String>> {
    let mut properties = HashMap::new();

    for (property_name, spec) in property_spec {
        // ignore this property if kind does not match
        // TODO: improve performance by sorting properties via kind
        if &property_name.kind != kind {
            continue;
        }

        // Ignore this configuration property if it is only available (specified via `as_of_version`)
        // in later versions than the one we're checking against.
        if Version::parse(spec.as_of_version.as_str())? > *product_version {
            continue;
        }

        // ignore completely if role is None
        // ignore this property if role does not match or is not required
        if let Some(property_roles) = &spec.roles {
            for property_role in property_roles {
                // role found?
                if Some(property_role.name.as_str()) == role && property_role.required {
                    // check for recommended value and matching version
                    if let Some(recommended) = &spec.recommended_values {
                        let property_value = get_property_value_for_version(
                            property_name,
                            recommended,
                            product_version,
                        )?;

                        properties.insert(property_name.name.clone(), property_value.value);

                        if let Some(property_dependencies) = &spec.depends_on {
                            let dependencies = get_config_dependencies_and_values(
                                property_spec,
                                product_version,
                                property_name,
                                &property_dependencies,
                            )?;

                            properties.extend(dependencies);
                        }
                    }
                    // no recommended values: Cli or Env single parameter
                    else if kind == &PropertyNameKind::Cli || kind == &PropertyNameKind::Env {
                        properties.insert(property_name.name.clone(), "".to_string());
                    }
                }
            }
        }
    }

    Ok(properties)
}

/// Collect all dependencies that are required based on user properties
///
/// # Arguments
///
/// * `property_spec` - map with property name as key and the corresponding property spec as value
/// * `user_config` - map with the user config names and according values
/// * `version` - the provided product version
/// * `kind` - property name kind provided by the user
///
pub(crate) fn get_matching_dependencies(
    property_spec: &HashMap<PropertyName, PropertySpec>,
    user_config: &HashMap<String, String>,
    version: &Version,
    kind: &PropertyNameKind,
) -> ValidationResult<HashMap<String, String>> {
    let mut user_dependencies = HashMap::new();
    for name in user_config.keys() {
        let property_name = PropertyName {
            name: name.clone(),
            kind: kind.clone(),
        };

        if let Some(property) = property_spec.get(&property_name) {
            if let Some(dependencies) = &property.depends_on {
                user_dependencies.extend(get_config_dependencies_and_values(
                    property_spec,
                    version,
                    &property_name,
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
    for property_dependency in property_dependencies {
        for property_dependency_name in &property_dependency.property_names {
            // the dependency should not differ in the kind
            if property_name.kind == property_dependency_name.kind {
                // if the dependency has a proposed value we are done
                if let Some(dependency_value) = &property_dependency.value {
                    dependencies.insert(
                        property_dependency_name.name.clone(),
                        dependency_value.clone(),
                    );
                }
                // we check the dependency for a recommended value
                if let Some(dependency_property) = property_spec.get(property_dependency_name) {
                    if let Some(recommended) = &dependency_property.recommended_values {
                        let recommended_value = get_property_value_for_version(
                            property_name,
                            recommended,
                            product_version,
                        )?;
                        dependencies.insert(
                            property_dependency_name.name.clone(),
                            recommended_value.value,
                        );
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
pub(crate) fn get_property_value_for_version(
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
