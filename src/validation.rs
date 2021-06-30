use crate::error::Error;
use crate::types::{Datatype, PropertySpec, Unit};
use std::fmt::Display;
use std::str::FromStr;

pub type ValidationResult<T> = Result<T, Error>;
/*
/// Returns the provided property_value if no validation errors appear
///
/// # Arguments
/// * `merged_properties` - merged user and property spec (matching role, kind etc.)
/// * `product_version` - version of the currently active product version
/// * `role` - the user role to validate against
/// * `property_name` - name of the property
/// * `property_value` - property value to be validated
///
pub(crate) fn validate(
    property_spec: &HashMap<PropertyName, PropertySpec>,
    config_spec: &ProductConfigSpecProperties,
    merged_properties: &HashMap<String, String>,
    product_version: &Version,
    role: Option<&str>,
    property_name: &PropertyName,
    property_value: &str,
) -> PropertyValidationResult {
    /*
    // a missing / wrong property stops us from doing any other validation
    let property = match property_spec.get(&property_name) {
        None => {
            return PropertyValidationResult::Error(
                property_value.to_string(),
                Error::PropertyNotFound {
                    property_name: property_name.clone(),
                },
            );
        }
        Some(opt) => opt,
    };

    let check_version = check_version_supported_or_deprecated(
        &property_name,
        product_version,
        &property.as_of_version[..],
        &property.deprecated_since,
    );

    if check_version.is_err() {
        return PropertyValidationResult::Error(
            property_value.to_string(),
            check_version.err().unwrap(),
        );
    }

    // for an empty value (""), ignore checks for the value (check_datatype, check_allowed_values..)
    if !property_value.is_empty() {
        let check_datatype = check_datatype(
            &config_spec.units,
            &property_name,
            property_value,
            &property.datatype,
        );
        if check_datatype.is_err() {
            return PropertyValidationResult::Error(
                property_value.to_string(),
                check_datatype.err().unwrap(),
            );
        }

        let check_allowed_values =
            check_allowed_values(&property_name, property_value, &property.allowed_values);
        if check_allowed_values.is_err() {
            return PropertyValidationResult::Error(
                property_value.to_string(),
                check_allowed_values.err().unwrap(),
            );
        }
    }

    let check_dependencies = check_dependencies(property_name, property, &merged_properties);
    if check_dependencies.is_err() {
        match check_dependencies.err() {
            None => {}
            Some(err) => {
                return match err {
                    Error::PropertyDependencyUserValueNotRequired { .. } => {
                        PropertyValidationResult::Warn(property_value.to_string(), err)
                    }
                    _ => PropertyValidationResult::Error(property_value.to_string(), err),
                }
            }
        }
    }

    let check_role = check_role(property_name, &property.roles, role);
    if check_role.is_err() {
        return PropertyValidationResult::Warn(
            property_value.to_string(),
            check_role.err().unwrap(),
        );
    }

    // was provided by recommended value?
    if Ok(true)
        == check_property_value_used(
            property_name,
            property_value,
            &property.recommended_values,
            &product_version,
        )
    {
        return PropertyValidationResult::RecommendedDefault(property_value.to_string());
    }

    // was provided by default value?
    if Ok(true)
        == check_property_value_used(
            property_name,
            property_value,
            &property.default_values,
            &product_version,
        )
    {
        return PropertyValidationResult::Default(property_value.to_string());
    }

    PropertyValidationResult::Valid(property_value.to_string())
    */
}


/// Check if the provided property spec is correct. Checks include:
/// - if default / recommended values match version, min / max, datatype, unit and regex
/// - if default / recommended values match allowed values if available
/// - if dependencies and required values match recommended values of that dependency
/// - if roles are available
///
/// # Arguments
/// * `config_spec` - map with unit name and respective regular expression to evaluate the datatype
/// * `property_spec` - map with property name as key and the corresponding property spec as value
///
pub(crate) fn validate_property_spec(
    config_spec: &ProductConfigSpecProperties,
    property_spec: &HashMap<PropertyName, PropertySpec>,
) -> ValidationResult<()> {
    for (name, spec) in property_spec {
        let as_of_version = semver_parse(&spec.as_of_version)?;

        // 1) check for default values
        if let Some(values) = &spec.default_values {
            // 1.1) check if a provided default version matches as_of_version
            util::get_property_value_for_version(name, values, &as_of_version)?;

            for val in values {
                // 1.2) check if default matches the allowed values
                check_allowed_values(name, &val.value, &spec.allowed_values)?;
                // 1.3) check if default values match datatype (min, max, unit...)
                check_datatype(&config_spec.units, name, &val.value, &spec.datatype)?
            }
        }

        // 2) check for recommended values
        if let Some(values) = &spec.recommended_values {
            // 2.1) check if a provided recommended version matches as_of_version
            util::get_property_value_for_version(name, values, &as_of_version)?;

            for val in values {
                // 2.2) check if recommended matches the allowed values
                check_allowed_values(name, &val.value, &spec.allowed_values)?;
                // 2.3) check if recommended values match datatype (min, max, unit...)
                check_datatype(&config_spec.units, name, &val.value, &spec.datatype)?
            }
        }

        // // prepare "user" data
        // let mut user_data = HashMap::new();
        // if let Some(dependencies) = &spec.depends_on {
        //     for dependency in dependencies {
        //         for dep_name in &dependency.property_names {
        //             if let Some(dependency_property) = property_spec.get(dep_name) {
        //                 if let Some(dependency_property_recommended) =
        //                     &dependency_property.recommended_values
        //                 {
        //                     let filtered_value = util::get_property_value_for_version(
        //                         &dep_name,
        //                         dependency_property_recommended,
        //                         &as_of_version,
        //                     )?;
        //
        //                     user_data.insert(dep_name.name.clone(), filtered_value.value.clone());
        //                 }
        //             } else {
        //                 return Err(Error::PropertyDependencyMissing {
        //                     property_name: name.clone(),
        //                     dependency: dependency.property_names.clone(),
        //                 });
        //             }
        //         }
        //     }
        // }
        // 3) check if dependency values are available and the recommended value matches the required one
        //check_dependencies(name, spec, &user_data)?;

        // 4) check if role available
        if spec.roles.is_none() {
            return Err(Error::PropertySpecRoleNotProvided { name: name.clone() });
        }
    }

    Ok(())
}
/// Check if the final used value corresponds to e.g. recommended or default values
///
/// # Arguments
///
/// * `property_name` - name of the property
/// * `property_value` - the final value used
/// * `property_values` - possible property names e.g. default or recommended values
/// * `product_version` - the provided product version
///
fn is(
    property_name: &PropertyName,
    property_value: &str,
    property_values: &Option<Vec<PropertyValueSpec>>,
    product_version: &Version,
) -> ValidationResult<bool> {
    if let Some(values) = property_values {
        let val = util::get_property_value_for_version(property_name, values, product_version)?;
        if val.value == property_value {
            return Ok(true);
        }
    }

    Ok(false)
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
            let from_version = semver_parse(from)?;

            if from_version > *product_version {
                continue;
            }
        }

        if let Some(to) = &value.to_version {
            let to_version = semver_parse(to)?;

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

/// Check if property role is available
///
/// # Arguments
///
/// * `property_name` - name of the property
/// * `roles` - roles provided in the property spec
/// * `config_role` - role provided by the user
///
fn check_role(
    property_name: &PropertyName,
    roles: &Option<Vec<Role>>,
    config_role: Option<&str>,
) -> ValidationResult<()> {
    if roles.is_none() {
        return Err(Error::PropertySpecRoleNotProvided {
            name: property_name.clone(),
        });
    }

    if config_role.is_none() {
        return Err(Error::PropertySpecRoleNotProvidedByUser {
            name: property_name.clone(),
        });
    }

    if let (Some(roles), Some(user_role)) = (roles, config_role) {
        for role in roles {
            if role.name == user_role {
                return Ok(());
            }
        }
    }

    Err(Error::PropertySpecRoleNotFound {
        name: property_name.clone(),
        role: config_role.unwrap().to_string(),
    })
}

/// Check if property version is supported or deprecated regarding the product version
///
/// # Arguments
///
/// * `property_name` - name of the property
/// * `product_version` - the current product version
/// * `property_version` - as of version of the provided config property
/// * `deprecated_since` - version from which point onwards the property is deprecated
///
fn check_version_supported_or_deprecated(
    property_name: &PropertyName,
    version: &Version,
    as_of_version: &str,
    deprecated_since: &Option<String>,
) -> ValidationResult<()> {
    let property_version = semver_parse(as_of_version)?;

    // compare version of the property and product version
    if property_version > *version {
        return Err(Error::VersionNotSupported {
            property_name: property_name.clone(),
            product_version: version.to_string(),
            required_version: property_version.to_string(),
        });
    }

    // check if requested property is deprecated
    if let Some(deprecated) = deprecated_since {
        let deprecated_since_version = semver_parse(deprecated.as_ref())?;

        if deprecated_since_version <= *version {
            return Err(Error::VersionDeprecated {
                property_name: property_name.clone(),
                product_version: version.to_string(),
                deprecated_version: deprecated_since_version.to_string(),
            });
        }
    }

    Ok(())
}

/// Check whether properties have provided dependencies and if they are contained the user properties
/// TODO: add dependency automatically if missing?
///
/// # Arguments
///
/// * `property_name` - name of the property
/// * `property` - the respective property spec
/// * `user_properties` - map with property name and potential value provided by user
///
fn check_dependencies(
    property_name: &PropertyName,
    property: &PropertySpec,
    user_properties: &HashMap<String, String>,
) -> ValidationResult<()> {
    // check if property has dependencies
    let property_dependencies = match &property.depends_on {
        None => return Ok(()),
        Some(dependencies) => dependencies,
    };

    // for each dependency, check if user_properties contain the property and the correct value
    // for property_dependency in property_dependencies {
    //     // check if we find any matches, otherwise return error after the loop
    //     let mut found_match = false;
    //     // for each property name provided within the dependency
    //     for dependency_property_name in &property_dependency.property_names {
    //         if !user_properties.contains_key(&dependency_property_name.name) {
    //             continue;
    //         }
    //
    //         found_match = true;
    //
    //         match (
    //             user_properties.get(&dependency_property_name.name),
    //             &property_dependency.value,
    //         ) {
    //             // no user value, no property value -> ok
    //             (None, None) => continue,
    //             // no user value but property value required -> error
    //             (None, Some(config_value)) => {
    //                 return Err(Error::PropertyDependencyUserValueMissing {
    //                     property_name: property_name.clone(),
    //                     dependency: dependency_property_name.name.clone(),
    //                     required_value: config_value.clone(),
    //                 })
    //             }
    //             // user value but no property value required -> error
    //             (Some(user_value), None) => {
    //                 return Err(Error::PropertyDependencyUserValueNotRequired {
    //                     property_name: property_name.clone(),
    //                     dependency: dependency_property_name.name.clone(),
    //                     user_value: user_value.clone(),
    //                 })
    //             }
    //             // both values available -> check if match
    //             (Some(user_value), Some(config_value)) => {
    //                 if user_value != config_value {
    //                     return Err(Error::PropertyDependencyValueInvalid {
    //                         property_name: property_name.clone(),
    //                         dependency: dependency_property_name.name.clone(),
    //                         user_value: user_value.clone(),
    //                         required_value: config_value.clone(),
    //                     });
    //                 }
    //             }
    //         }
    //     }

    //     if !found_match {
    //         // TODO: Error or just add the correct dependency?
    //         return Err(Error::PropertyDependencyMissing {
    //             property_name: property_name.clone(),
    //             dependency: property_dependency.property_names.clone(),
    //         });
    //     }
    // }

    Ok(())
}
*/
/// Check if property value fits the provided datatype
/// # Arguments
///
/// * `config_spec_units` - map with unit name and respective regular expression to evaluate the datatype
/// * `property_name` - name of the property
/// * `property_value` - property value to be validated
/// * `datatype` - property datatype containing min/max bounds, units etc.
///
pub(crate) fn check_datatype(
    property: &PropertySpec,
    name: &str,
    value: &str,
) -> ValidationResult<()> {
    match &property.datatype {
        Datatype::Bool => {
            check_datatype_scalar::<bool>(name, value, &None, &None)?;
        }
        Datatype::Integer { min, max, .. } => {
            check_datatype_scalar::<i64>(name, value, min, max)?;
        }
        Datatype::Float { min, max, .. } => {
            check_datatype_scalar::<f64>(name, value, min, max)?;
        }
        Datatype::String { min, max, unit, .. } => {
            check_datatype_string(name, value, min, max, unit)?;
        }
        Datatype::Array { .. } => {
            // TODO: implement logic for array type
        }
    }
    Ok(())
}

/// Check if property value is in allowed values
/// # Arguments
///
/// * `property_name` - name of the property
/// * `property_value` - property value to be validated
/// * `allowed_values` - vector of allowed values
///
// fn check_allowed_values(
//     property_name: &PropertyName,
//     property_value: &str,
//     allowed_values: &Option<Vec<String>>,
// ) -> ValidationResult<()> {
//     if allowed_values.is_some() {
//         let allowed_values = allowed_values.clone().unwrap();
//         if !allowed_values.is_empty() && !allowed_values.contains(&property_value.to_string()) {
//             return Err(Error::PropertyValueNotInAllowedValues {
//                 property_name: property_name.clone(),
//                 value: property_value.to_string(),
//                 allowed_values,
//             });
//         }
//     }
//     Ok(())
// }

/// Returns the provided scalar parameter value of type T (i16, i32, i64, f32, f62-..) if no parsing errors appear
///
/// # Arguments
///
/// * `name` - name of the property
/// * `value` - the value belonging to the property to be validated
/// * `min` - minimum value specified
/// * `max` - maximum value specified
///
fn check_datatype_scalar<T>(
    name: &str,
    value: &str,
    min: &Option<String>,
    max: &Option<String>,
) -> ValidationResult<T>
where
    T: FromStr + std::cmp::PartialOrd + Display + Copy,
{
    // check if config_value fits datatype
    let val: T = parse::<T>(name, value)?;
    // check min bound
    check_bound(name, val, min, min_bound)?;
    // check max bound
    check_bound(name, val, max, max_bound)?;

    Ok(val)
}

/// Returns the provided text parameter value of type T if no parsing errors appear
///
/// # Arguments
///
/// * `name` - name of the property
/// * `value` - the value belonging to the property to be validated
/// * `min` - minimum value specified
/// * `max` - maximum value specified
/// * `unit` - provided unit to get the regular expression to parse the property_value
///
fn check_datatype_string(
    name: &str,
    value: &str,
    min: &Option<String>,
    max: &Option<String>,
    unit: &Option<Unit>,
) -> ValidationResult<()> {
    let len: usize = value.len();
    check_bound::<usize>(name, len, min, min_bound)?;
    check_bound::<usize>(name, len, max, max_bound)?;

    if let Some(unit) = unit {
        if !unit.regex.is_match(value) {
            return Err(Error::DatatypeRegexNotMatching {
                property_name: name.to_string(),
                value: value.to_string(),
            });
        }
    }

    Ok(())
}

/// Check if value is out of min bound
///
/// # Arguments
///
/// * `val` - value to be validated
/// * `min` - min border (exclusive)
///
fn min_bound<T>(val: T, min: T) -> bool
where
    T: FromStr + std::cmp::PartialOrd + Display + Copy,
{
    val < min
}

/// Check if value is out of max bound
///
/// # Arguments
///
/// * `val` - value to be validated
/// * `max` - max border (exclusive)
///
fn max_bound<T>(val: T, min: T) -> bool
where
    T: FromStr + std::cmp::PartialOrd + Display + Copy,
{
    val > min
}

/// Check if a value is inside a certain bound
///
/// # Arguments
///
/// * `property_name` - name of the property
/// * `value` - value to be validated
/// * `bound` - upper/lower bound
/// * `check_out_of_bound` - the method to check against the bound
///
fn check_bound<T>(
    name: &str,
    value: T,
    bound: &Option<String>,
    check_out_of_bound: fn(T, T) -> bool,
) -> ValidationResult<T>
where
    T: FromStr + std::cmp::PartialOrd + Display + Copy,
{
    if let Some(bound) = bound {
        let bound: T = parse::<T>(name, bound.as_str())?;
        if check_out_of_bound(value, bound) {
            return Err(Error::PropertyValueOutOfBounds {
                property_name: name.to_string(),
                received: value.to_string(),
                expected: bound.to_string(),
            });
        }
    }

    Ok(value)
}

/// Parse a value to a certain datatype and throw error if parsing not possible
///
/// # Arguments
///
/// * `property_name` - name of the property
/// * `to_parse` - value to be parsed into a certain T
///
fn parse<T: FromStr>(name: &str, to_parse: &str) -> Result<T, Error> {
    match to_parse.parse::<T>() {
        Ok(to_parse) => Ok(to_parse),
        Err(_) => {
            return Err(Error::DatatypeNotMatching {
                property_name: name.to_string(),
                value: to_parse.to_string(),
                datatype: std::any::type_name::<T>().to_string(),
            })
        }
    }
}

/*
#[cfg(test)]
mod tests {
    macro_rules! hashmap {
        ($( $key: expr => $val: expr ),*) => {{
             let mut map = ::std::collections::HashMap::new();
             $( map.insert($key, $val); )*
             map
        }}
    }

    use crate::error::Error;
    use crate::reader::ConfigJsonReader;
    use crate::types::{Datatype, PropertyName, PropertyNameKind, Role};
    use crate::validation::{
        check_allowed_values, check_datatype, check_dependencies, check_role,
        check_version_supported_or_deprecated,
    };
    use crate::ProductConfigSpec;
    use rstest::*;
    use semver::Version;
    use std::collections::HashMap;

    const ENV_INTEGER_PORT_MIN_MAX: &str = "ENV_INTEGER_PORT_MIN_MAX";
    const ENV_PROPERTY_STRING_MEMORY: &str = "ENV_PROPERTY_STRING_MEMORY";
    const ENV_SSL_CERTIFICATE_PATH: &str = "ENV_SSL_CERTIFICATE_PATH";
    const ENV_SSL_ENABLED: &str = "ENV_SSL_ENABLED";
    const CONF_SSL_ENABLED: &str = "conf.ssl.enabled";
    const ENV_ALLOWED_VALUES: &str = "ENV_ALLOWED_VALUES";
    const ENV_VAR_FLOAT: &str = "ENV_VAR_FLOAT";

    const CONFIG_FILE: &str = "env.sh";
    const CONFIG_FILE_2: &str = "my.config";

    const V_1_5_0: &str = "1.5.0";
    const V_1_0_0: &str = "1.0.0";
    const V_0_5_0: &str = "0.5.0";
    const V_0_1_0: &str = "0.1.0";

    fn get_conf_property_name(name: &str, file: &str) -> PropertyName {
        PropertyName {
            name: name.to_string(),
            kind: PropertyNameKind::File(file.to_string()),
        }
    }

    fn get_product_config() -> ProductConfigSpec {
        ProductConfigSpec::new(ConfigJsonReader::new(
            "data/test_config_spec.json",
            "data/test_property_spec.json",
        ))
        .unwrap()
    }

    #[rstest]
    #[case(get_conf_property_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE), V_1_0_0, V_0_5_0, None, Ok(()))]
    #[case(get_conf_property_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE), V_0_1_0, V_1_0_0, Some(V_0_5_0.to_string()),
            Err(Error::VersionNotSupported { property_name: property_name.clone(), product_version: V_0_1_0.to_string(), required_version: V_1_0_0.to_string() }))]
    #[case(get_conf_property_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE), V_1_5_0, V_0_5_0, Some(V_1_0_0.to_string()),
            Err(Error::VersionDeprecated { property_name: property_name.clone(), product_version: V_1_5_0.to_string(), deprecated_version: V_1_0_0.to_string() }))]
    #[trace]
    fn test_check_version_supported_or_deprecated(
        #[case] property_name: PropertyName,
        #[case] product_version: &str,
        #[case] property_version: &str,
        #[case] deprecated_since: Option<String>,
        #[case] expected: Result<(), Error>,
    ) {
        let result = check_version_supported_or_deprecated(
            &property_name,
            &Version::parse(product_version).unwrap(),
            property_version,
            &deprecated_since,
        );

        assert_eq!(result, expected)
    }

    const ROLE_1: &str = "role_1";
    const ROLE_2: &str = "role_2";

    #[rstest]
    #[case(
        &get_conf_property_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE),
        Some(ROLE_1),
        Ok(())
    )]
    #[case(
        &get_conf_property_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE),
        Some(ROLE_2),
        Ok(())
    )]
    #[case(
        &get_conf_property_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE),
        None,
        Err(Error::PropertySpecRoleNotProvidedByUser { name: get_conf_property_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE) })
    )]
    #[trace]
    fn test_check_role(
        #[case] property_name: &PropertyName,
        #[case] role: Option<&str>,
        #[case] expected: Result<(), Error>,
    ) {
        let property_roles = Some(vec![
            Role {
                name: ROLE_1.to_string(),
                required: true,
            },
            Role {
                name: ROLE_2.to_string(),
                required: false,
            },
        ]);

        let result = check_role(property_name, &property_roles, role);

        assert_eq!(result, expected)
    }

    #[rstest]
    #[case(
        &get_conf_property_name(ENV_SSL_CERTIFICATE_PATH, CONFIG_FILE),
        hashmap!{
            ENV_SSL_CERTIFICATE_PATH.to_string() => "some/path/to/certificate".to_string()
        },
        Err(Error::PropertyDependencyMissing {
            property_name: get_conf_property_name(ENV_SSL_CERTIFICATE_PATH, CONFIG_FILE),
            dependency: vec![
                PropertyName { name: ENV_SSL_ENABLED.to_string(), kind: PropertyNameKind::File(CONFIG_FILE.to_string()) },
                PropertyName { name: CONF_SSL_ENABLED.to_string(), kind: PropertyNameKind::File(CONFIG_FILE_2.to_string()) }
            ]
        })
    )]
    #[case(
        &get_conf_property_name(ENV_SSL_CERTIFICATE_PATH, CONFIG_FILE),
        hashmap!{
            ENV_SSL_CERTIFICATE_PATH.to_string() => "some/path/to/certificate".to_string(),
            ENV_SSL_ENABLED.to_string() => "false".to_string()
        },
        Err(Error::PropertyDependencyValueInvalid {
            property_name: get_conf_property_name(ENV_SSL_CERTIFICATE_PATH, CONFIG_FILE),
            dependency: "ENV_SSL_ENABLED".to_string(),
            user_value: "false".to_string(),
            required_value: "true".to_string()
        })
    )]
    #[case(
        &get_conf_property_name(ENV_SSL_CERTIFICATE_PATH, CONFIG_FILE),
        hashmap!{
            ENV_SSL_CERTIFICATE_PATH.to_string() => "some/path/to/certificate".to_string(),
            ENV_SSL_ENABLED.to_string() => "true".to_string()
        },
        Ok(())
    )]
    #[trace]
    fn test_check_dependencies(
        #[case] property_name: &PropertyName,
        #[case] user_properties: HashMap<String, String>,
        #[case] expected: Result<(), Error>,
    ) {
        let product_config = get_product_config();
        let property_spec = product_config.property_specs.get(&property_name).unwrap();

        let result = check_dependencies(&property_name, property_spec, &user_properties);

        assert_eq!(result, expected)
    }

    const MIN_PORT: &str = "1";
    const MAX_PORT: &str = "65535";
    const PORT_CORRECT: &str = "12345";
    const PORT_BAD_DATATYPE: &str = "123aaa";
    const PORT_OUT_OF_BOUNDS: &str = "77777";

    const MEMORY_CORRECT_MB: &str = "512mb";
    const MEMORY_CORRECT_GB: &str = "2gb";
    const MEMORY_MISSING_UNIT: &str = "512";

    const FLOAT_CORRECT: &str = "87.2123";
    const FLOAT_BAD: &str = "100,0";

    #[rstest]
    #[case(
        &get_conf_property_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE),
        PORT_CORRECT,
        &Datatype::Integer{ min: Some(MIN_PORT.to_string()), max: Some(MAX_PORT.to_string()), unit: Some("port".to_string()), accepted_units: None, default_unit:None },
        Ok(())
    )]
    #[case(
        &get_conf_property_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE),
        PORT_BAD_DATATYPE,
        &Datatype::Integer{ min: Some(MIN_PORT.to_string()), max: Some(MAX_PORT.to_string()), unit: Some("port".to_string()), accepted_units: None, default_unit:None },
        Err(Error::DatatypeNotMatching { property_name: get_conf_property_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE), value: PORT_BAD_DATATYPE.to_string(), datatype: "i64".to_string() })
    )]
    #[case(
        &get_conf_property_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE),
        PORT_OUT_OF_BOUNDS,
        &Datatype::Integer{ min: Some(MIN_PORT.to_string()), max: Some(MAX_PORT.to_string()), unit: Some("port".to_string()), accepted_units: None, default_unit:None },
        Err(Error::PropertyValueOutOfBounds { property_name: get_conf_property_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE), received: PORT_OUT_OF_BOUNDS.to_string(), expected: MAX_PORT.to_string() })
    )]
    #[case(
        &get_conf_property_name(ENV_PROPERTY_STRING_MEMORY, CONFIG_FILE),
        MEMORY_CORRECT_MB,
        &Datatype::String{ min: None, max: None, unit: Some("memory".to_string()), accepted_units: None, default_unit:None },
        Ok(())
    )]
    #[case(
        &get_conf_property_name(ENV_PROPERTY_STRING_MEMORY, CONFIG_FILE),
        MEMORY_CORRECT_GB,
        &Datatype::String{ min: None, max: None, unit: Some("memory".to_string()), accepted_units: None, default_unit:None },
        Ok(())
    )]
    #[case(
        &get_conf_property_name(ENV_PROPERTY_STRING_MEMORY, CONFIG_FILE),
        MEMORY_MISSING_UNIT,
        &Datatype::String{ min: None, max: None, unit: Some("memory".to_string()), accepted_units: None, default_unit:None },
        Err(Error::DatatypeRegexNotMatching { property_name: get_conf_property_name(ENV_PROPERTY_STRING_MEMORY, CONFIG_FILE), value: MEMORY_MISSING_UNIT.to_string() })
    )]
    #[case(
        &get_conf_property_name(ENV_VAR_FLOAT, CONFIG_FILE),
        FLOAT_CORRECT,
        &Datatype::Float{ min: Some("0.0".to_string()), max: Some("100.0".to_string()), unit: None, accepted_units: None, default_unit:None },
        Ok(())
    )]
    #[case(
        &get_conf_property_name(ENV_VAR_FLOAT, CONFIG_FILE),
        FLOAT_BAD,
        &Datatype::Float{ min: Some("0.0".to_string()), max: Some("100.0".to_string()), unit: None, accepted_units: None, default_unit:None },
        Err(Error::DatatypeNotMatching { property_name: get_conf_property_name(ENV_VAR_FLOAT, CONFIG_FILE), value: FLOAT_BAD.to_string(), datatype: "f64".to_string() })
    )]
    #[trace]
    fn test_check_datatype(
        #[case] property_name: &PropertyName,
        #[case] property_value: &str,
        #[case] datatype: &Datatype,
        #[case] expected: Result<(), Error>,
    ) {
        let config_spec_units = get_product_config().config_spec.units;

        let result = check_datatype(&config_spec_units, property_name, property_value, &datatype);

        assert_eq!(result, expected)
    }

    const ALLOWED_VALUE_1: &str = "allowed_value_1";
    const ALLOWED_VALUE_2: &str = "allowed_value_2";
    const ALLOWED_VALUE_3: &str = "allowed_value_3";
    const NOT_ALLOWED_VALUE: &str = "not_allowed_value";

    #[rstest]
    #[case(
        &get_conf_property_name(ENV_ALLOWED_VALUES, CONFIG_FILE),
        ALLOWED_VALUE_1,
        Some(vec![ALLOWED_VALUE_1.to_string(), ALLOWED_VALUE_2.to_string(), ALLOWED_VALUE_3.to_string()]),
        Ok(())
    )]
    #[case(
        &get_conf_property_name(ENV_ALLOWED_VALUES, CONFIG_FILE),
        NOT_ALLOWED_VALUE,
        Some(vec![ALLOWED_VALUE_1.to_string(), ALLOWED_VALUE_2.to_string(), ALLOWED_VALUE_3.to_string()]),
        Err(Error::PropertyValueNotInAllowedValues {
            property_name: get_conf_property_name(ENV_ALLOWED_VALUES, CONFIG_FILE),
            value: NOT_ALLOWED_VALUE.to_string(),
            allowed_values: vec![ALLOWED_VALUE_1.to_string(), ALLOWED_VALUE_2.to_string(), ALLOWED_VALUE_3.to_string() ]
        })
    )]
    #[trace]
    fn test_check_allowed_values(
        #[case] property_name: &PropertyName,
        #[case] property_value: &str,
        #[case] allowed_values: Option<Vec<String>>,
        #[case] expected: Result<(), Error>,
    ) {
        let result = check_allowed_values(property_name, property_value, &allowed_values);

        assert_eq!(result, expected)
    }
}
*/
