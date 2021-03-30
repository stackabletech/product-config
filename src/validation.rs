//type ConfigValidatorResult<T> = std::result::Result<T, error::Error>;

use crate::error::Error;
use crate::types::{ConfigOption, Datatype, OptionName};
use crate::ProductConfigResult;
use regex::Regex;
use semver::Version;
use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;

type ConfigValidationResult<T> = Result<T, Error>;

/// Returns the provided option_value if no validation errors appear
///
/// # Arguments
///
/// * `product_version` - version of the currently active product version
/// * `option_name` - name of the config option (config property or environmental variable)
/// * `option_value` - config option value to be validated; Option.None means missing, Option<""> will avoid some checks and set option to empty
///
/// # Examples
///
/// ```
/// ```
pub fn validate(
    config_options: &HashMap<OptionName, ConfigOption>,
    config_setting_units: &HashMap<String, Regex>,
    product_version: &str,
    option_name: &OptionName,
    option_value: Option<String>,
) -> ProductConfigResult {
    // a missing / wrong config option stops us from doing any other validation
    let config_option = match config_options.get(&option_name) {
        None => {
            return ProductConfigResult::Error(Error::ConfigOptionNotFound {
                option_name: option_name.clone(),
            });
        }
        Some(opt) => opt,
    };

    let value = match option_value {
        None => {
            // value missing is just an error
            return ProductConfigResult::Error(Error::ConfigValueMissing {
                option_name: option_name.clone(),
            });
        }
        Some(val) => val,
    };

    // checks for config option
    let check_version = check_version_supported_or_deprecated(
        &option_name,
        product_version,
        &config_option.as_of_version[..],
        &config_option.deprecated_since,
    );

    if check_version.is_err() {
        return ProductConfigResult::Error(check_version.err().unwrap());
    }

    // for an empty value (""), ignore checks for the value (check_datatype, check_allowed_values..)
    if !value.is_empty() {
        let check_datatype = check_datatype(
            config_setting_units,
            &option_name,
            value.as_str(),
            &config_option.datatype,
        );
        if check_datatype.is_err() {
            return ProductConfigResult::Error(check_datatype.err().unwrap());
        }

        let check_allowed_values =
            check_allowed_values(&option_name, value.as_str(), &config_option.allowed_values);
        if check_allowed_values.is_err() {
            return ProductConfigResult::Error(check_allowed_values.err().unwrap());
        }
    }

    ProductConfigResult::Valid(value)
}

// pub fn validate_all(
//     config_options: &HashMap<OptionName, ConfigOption>,
//     config_setting_units: &HashMap<String, Regex>,
//     product_version: &str,
//     options: &HashMap<OptionName, Option<String>>,
// ) -> HashMap<OptionName, ProductConfigResult> {
//     let mut result = HashMap::new();
//     for (option_name, option_value) in options {
//         // single option validation
//         validate(
//             config_options,
//             config_setting_units,
//             product_version,
//             option_name,
//             option_value.clone(),
//         );
//     }
//
//     // additional dependency validation
//     check_dependencies(config_options, &options)?;
//
//     result
// }

// /// Check if default or recommended values are available
// /// Check their datatype and version match
// /// Check if dependencies are available
// fn check_config_options(config_options: &HashMap<OptionName, ConfigOption>) {
//     // build local name -> value map where value is either default or recommended value
//     let mut local_config: HashMap<String, String> = HashMap::new();
//     for (option_name, config_option) in config_options {
//         // check if there are values matching the version
//         let version = &config_option.as_of_version;
//
//         if let Some(recommended) = &config_option.recommended_values {
//             for value in recommended {
//                 if let Some(from_version) = &value.from_version {
//
//                 }
//             }
//
//
//         } else if let Some(default) = &config_option.default_values {
//         }
//     }
// }

/// Check if config option version is supported or deprecated regarding the product version
/// # Arguments
///
/// * `option_name` - name of the config option (config property or environmental variable)
/// * `product_version` - product / controller version
/// * `option_version` - as of version of the provided config option
/// * `deprecated_since` - version from which point onwards the option is deprecated
///
fn check_version_supported_or_deprecated(
    option_name: &OptionName,
    product_version: &str,
    option_version: &str,
    deprecated_since: &Option<String>,
) -> ConfigValidationResult<()> {
    let product_version = Version::parse(product_version)?;
    let option_version = Version::parse(option_version)?;

    // compare version of the config option and product / controller version
    if option_version > product_version {
        return Err(Error::VersionNotSupported {
            option_name: option_name.clone(),
            product_version: product_version.to_string(),
            required_version: option_version.to_string(),
        });
    }

    // check if requested config option is deprecated
    if let Some(deprecated) = deprecated_since {
        let deprecated_since_version = Version::parse(deprecated.as_ref())?;

        if deprecated_since_version <= product_version {
            return Err(Error::VersionDeprecated {
                option_name: option_name.clone(),
                product_version: product_version.to_string(),
                deprecated_version: deprecated_since_version.to_string(),
            });
        }
    }

    Ok(())
}

/// Check whether options have provided dependencies and if they are contained / set in the options map
///
/// # Arguments
///
/// * `options` - Map with config_option names and config_option values
///
fn check_dependencies(
    config_options: &HashMap<OptionName, ConfigOption>,
    user_options: &HashMap<OptionName, Option<String>>,
) -> ConfigValidationResult<()> {
    for option_name in user_options.keys() {
        // check if provided option_name has dependencies in config
        let dependencies = match config_options.get(option_name) {
            None => continue,
            Some(dep) => match &dep.depends_on {
                None => continue,
                Some(dependencies) => dependencies,
            },
        };

        // for each dependency check names
        for dependency in dependencies {
            for dependency_name in &dependency.option_names {
                // TODO: for now we search only for dependencies with the same kind (e.g. both env variables)
                if dependency_name.kind != option_name.kind {
                    continue;
                }
                // check if name is available in user options
                if let Some(user_value) = user_options.get(&dependency_name) {
                    // check if provided required values available
                    if let (Some(user_value), Some(dependency_value)) =
                        (user_value, &dependency.value)
                    {
                        if user_value != dependency_value {
                            return Err(Error::ConfigDependencyValueInvalid {
                                option_name: option_name.clone(),
                                dependency: dependency_name.name.clone(),
                                option_value: user_value.clone(),
                                required_value: dependency_value.clone(),
                            });
                        }
                    }
                } else {
                    return Err(Error::ConfigDependencyMissing {
                        option_name: option_name.clone(),
                        dependency: dependency_name.name.clone(),
                    });
                }
            }
        }
    }

    Ok(())
}

/// Check if option value fits the provided datatype
/// # Arguments
///
/// * `option_name` - name of the config option (config property or environmental variable)
/// * `option_value` - config option value to be validated
/// * `datatype` - containing min/max bounds, units etc.
///
fn check_datatype(
    config_setting_units: &HashMap<String, Regex>,
    option_name: &OptionName,
    option_value: &str,
    datatype: &Datatype,
) -> ConfigValidationResult<()> {
    match datatype {
        Datatype::Bool => {
            check_datatype_scalar::<bool>(option_name, option_value, &None, &None)?;
        }
        Datatype::Integer { min, max, .. } => {
            check_datatype_scalar::<i64>(option_name, option_value, min, max)?;
        }
        Datatype::Float { min, max, .. } => {
            check_datatype_scalar::<f64>(option_name, option_value, min, max)?;
        }
        Datatype::String { min, max, unit, .. } => {
            check_datatype_string(
                config_setting_units,
                option_name,
                option_value,
                min,
                max,
                unit,
            )?;
        }
        Datatype::Array { .. } => {
            // TODO: implement logic for array type
        }
    }
    Ok(())
}

/// Check if option value is in allowed values
/// # Arguments
///
/// * `option_name` - name of the config option (config property or environmental variable)
/// * `option_value` - config option value to be validated
/// * `allowed_values` - vector of allowed values
///
fn check_allowed_values(
    option_name: &OptionName,
    option_value: &str,
    allowed_values: &Option<Vec<String>>,
) -> ConfigValidationResult<()> {
    if allowed_values.is_some() {
        let allowed_values = allowed_values.clone().unwrap();
        if !allowed_values.is_empty() && !allowed_values.contains(&option_value.to_string()) {
            return Err(Error::ConfigValueNotInAllowedValues {
                option_name: option_name.clone(),
                value: option_value.to_string(),
                allowed_values,
            });
        }
    }
    Ok(())
}

/// Returns the provided scalar parameter value of type T (i16, i32, i64, f32, f62-..) if no parsing errors appear
///
/// # Arguments
///
/// * `option_name` - name of the config option (config property or environmental variable)
/// * `option_value` - config option value to be validated
/// * `min` - minimum value specified in config_option.data_format.min
/// * `max` - maximum value specified in config_option.data_format.max
///
fn check_datatype_scalar<T>(
    option_name: &OptionName,
    option_value: &str,
    min: &Option<String>,
    max: &Option<String>,
) -> ConfigValidationResult<T>
where
    T: FromStr + std::cmp::PartialOrd + Display + Copy,
{
    // check if config_value fits datatype
    let val: T = parse::<T>(option_name, option_value)?;
    // check min bound
    check_bound(option_name, val, min, min_bound)?;
    // check max bound
    check_bound(option_name, val, max, max_bound)?;

    Ok(val)
}

/// Returns the provided text parameter value of type T if no parsing errors appear
///
/// # Arguments
///
/// * `option_name` - name of the config option (config property or environmental variable)
/// * `option_value` - config option value to be validated
/// * `min` - minimum value specified in config_option.data_format.min
/// * `max` - maximum value specified in config_option.data_format.max
/// * `unit` - provided unit to get the regular expression to parse the option_value
///
fn check_datatype_string(
    config_setting_units: &HashMap<String, Regex>,
    option_name: &OptionName,
    option_value: &str,
    min: &Option<String>,
    max: &Option<String>,
    unit: &Option<String>,
) -> ConfigValidationResult<()> {
    let len: usize = option_value.len();
    check_bound::<usize>(option_name, len, min, min_bound)?;
    check_bound::<usize>(option_name, len, max, max_bound)?;

    if let Some(unit_name) = unit {
        match config_setting_units.get(unit_name.as_str()) {
            None => {
                return Err(Error::UnitSettingNotFound {
                    option_name: option_name.clone(),
                    unit: unit_name.clone(),
                })
            }
            Some(regex) => {
                if !regex.is_match(option_value) {
                    return Err(Error::DatatypeRegexNotMatching {
                        option_name: option_name.clone(),
                        value: option_value.to_string(),
                    });
                }
            }
        }
    } else {
        return Err(Error::UnitNotProvided {
            option_name: option_name.clone(),
        });
    }

    Ok(())
}

/// Check if value is out of min bound
fn min_bound<T>(val: T, min: T) -> bool
where
    T: FromStr + std::cmp::PartialOrd + Display + Copy,
{
    val < min
}

/// Check if value is out of max bound
fn max_bound<T>(val: T, min: T) -> bool
where
    T: FromStr + std::cmp::PartialOrd + Display + Copy,
{
    val > min
}

/// Check if a value is inside a certain bound
fn check_bound<T>(
    option_name: &OptionName,
    value: T,
    bound: &Option<String>,
    check_out_of_bound: fn(T, T) -> bool,
) -> ConfigValidationResult<T>
where
    T: FromStr + std::cmp::PartialOrd + Display + Copy,
{
    if let Some(bound) = bound {
        let bound: T = parse::<T>(option_name, bound.as_str())?;
        if check_out_of_bound(value, bound) {
            return Err(Error::ConfigValueOutOfBounds {
                option_name: option_name.clone(),
                received: value.to_string(),
                expected: bound.to_string(),
            });
        }
    }

    Ok(value)
}

/// Parse a value to a certain datatype and throw error if parsing not possible
fn parse<T: FromStr>(option_name: &OptionName, to_parse: &str) -> Result<T, Error> {
    match to_parse.parse::<T>() {
        Ok(to_parse) => Ok(to_parse),
        Err(_) => {
            return Err(Error::DatatypeNotMatching {
                option_name: option_name.clone(),
                value: to_parse.to_string(),
                datatype: std::any::type_name::<T>().to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::error::Error;
    use crate::types::{Datatype, OptionKind, OptionName};
    use crate::validation::{check_datatype, check_version_supported_or_deprecated};
    use rstest::*;

    const NAME: &str = "test_name";
    const CONFIG_VALUE: &str = "test.config";
    const V_1_5_0: &str = "1.5.0";
    const V_1_0_0: &str = "1.0.0";
    const V_0_5_0: &str = "0.5.0";
    const V_0_1_0: &str = "0.1.0";

    fn get_option_name() -> OptionName {
        OptionName {
            name: NAME.to_string(),
            kind: OptionKind::Conf,
            config_file: CONFIG_VALUE.to_string(),
        }
    }

    #[rstest(
        option_name,
        product_version,
        option_version,
        deprecated_since,
        expected,
        case(get_option_name(), V_1_0_0, V_0_5_0, None, Ok(())),
        case(get_option_name(), V_0_1_0, V_1_0_0, Some(V_0_5_0.to_string()),
            Err(Error::VersionNotSupported { option_name: option_name.clone(), product_version: V_0_1_0.to_string(), required_version: V_1_0_0.to_string() })),
        case(get_option_name(), V_1_5_0, V_0_5_0, Some(V_1_0_0.to_string()),
            Err(Error::VersionDeprecated { option_name: option_name.clone(), product_version: V_1_5_0.to_string(), deprecated_version: V_1_0_0.to_string() })),
        ::trace
    )]
    fn test_check_version_supported_or_deprecated(
        option_name: OptionName,
        product_version: &str,
        option_version: &str,
        deprecated_since: Option<String>,
        expected: Result<(), Error>,
    ) {
        let result = check_version_supported_or_deprecated(
            &option_name,
            product_version,
            option_version,
            &deprecated_since,
        );

        assert_eq!(result, expected)
    }

    // #[rstest(
    //     config_setting_units,
    //     option_name,
    //     option_value,
    //     datatype,
    //     expected,
    //     case(get_option_name(), V_1_0_0, V_0_5_0, None, Ok(())),
    //     ::trace
    // )]
    // fn test_check_datatype(
    //     config_setting_units: &HashMap<String, Regex>,
    //     option_name: &OptionName,
    //     option_value: &str,
    //     datatype: &Datatype,
    //     expected: Result<(), Error>,
    // ) {
    //     let result = check_datatype(
    //         &option_name,
    //         product_version,
    //         option_version,
    //         &deprecated_since,
    //     );
    //
    //     assert_eq!(result, expected)
    // }
}
