use crate::error::Error;
use crate::types::{ConfigName, ConfigOption, Datatype, OptionValue, Role};
use crate::util;
use crate::ConfigOptionValidationResult;
use regex::Regex;
use semver::Version;
use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;

pub type ValidationResult<T> = Result<T, Error>;

/// Returns the provided option_value if no validation errors appear
///
/// # Arguments
/// * `config_options` - map with ConfigName as key and the corresponding ConfigOption as value
/// * `config_setting_units` - map with unit name and respective regular expression to evaluate the datatype
/// * `merged_config_options` - merged user and config options (matching role, kind etc.)
/// * `product_version` - version of the currently active product version
/// * `role` - the user role to validate against
/// * `option_name` - name of the config option (config property or environmental variable)
/// * `option_value` - config option value to be validated; Option.None means missing, Option<""> will avoid some checks and set option to empty
///
pub fn validate(
    config_options: &HashMap<ConfigName, ConfigOption>,
    config_setting_units: &HashMap<String, Regex>,
    merged_config_options: &HashMap<String, String>,
    product_version: &Version,
    role: Option<&str>,
    option_name: &ConfigName,
    option_value: &str,
) -> ConfigOptionValidationResult {
    // a missing / wrong config option stops us from doing any other validation
    let config_option = match config_options.get(&option_name) {
        None => {
            return ConfigOptionValidationResult::Error(Error::ConfigOptionNotFound {
                option_name: option_name.clone(),
            });
        }
        Some(opt) => opt,
    };

    // checks for config option
    let check_version = check_version_supported_or_deprecated(
        &option_name,
        product_version,
        &config_option.as_of_version[..],
        &config_option.deprecated_since,
    );

    if check_version.is_err() {
        return ConfigOptionValidationResult::Error(check_version.err().unwrap());
    }

    // for an empty value (""), ignore checks for the value (check_datatype, check_allowed_values..)
    if !option_value.is_empty() {
        let check_datatype = check_datatype(
            config_setting_units,
            &option_name,
            option_value,
            &config_option.datatype,
        );
        if check_datatype.is_err() {
            return ConfigOptionValidationResult::Error(check_datatype.err().unwrap());
        }

        let check_allowed_values =
            check_allowed_values(&option_name, option_value, &config_option.allowed_values);
        if check_allowed_values.is_err() {
            return ConfigOptionValidationResult::Error(check_allowed_values.err().unwrap());
        }
    }

    let check_dependencies = check_dependencies(option_name, config_option, &merged_config_options);
    if check_dependencies.is_err() {
        match check_dependencies.err() {
            None => {}
            Some(err) => {
                return match err {
                    Error::ConfigDependencyUserValueNotRequired { .. } => {
                        ConfigOptionValidationResult::Warn(option_value.to_string(), err)
                    }
                    _ => ConfigOptionValidationResult::Error(err),
                }
            }
        }
    }

    let check_role = check_role(option_name, &config_option.roles, role);
    if check_role.is_err() {
        return ConfigOptionValidationResult::Warn(
            option_value.to_string(),
            check_role.err().unwrap(),
        );
    }

    // was provided by recommended value?
    if Ok(true)
        == check_option_value_used(
            option_name,
            option_value,
            &config_option.recommended_values,
            &product_version,
        )
    {
        return ConfigOptionValidationResult::RecommendedDefault(option_value.to_string());
    }

    // was provided by default value?
    if Ok(true)
        == check_option_value_used(
            option_name,
            option_value,
            &config_option.default_values,
            &product_version,
        )
    {
        return ConfigOptionValidationResult::Default(option_value.to_string());
    }

    ConfigOptionValidationResult::Valid(option_value.to_string())
}

/// Check if the provided config items are correct. Checks include:
/// - if default / recommended values match version, min / max, datatype, unit and regex
/// - if default / recommended values match allowed values if available
/// - if dependencies and required values match recommended values of that dependency
/// - if roles are available
///
/// # Arguments
/// * `config_options` - map with ConfigName as key and the corresponding ConfigOption as value
/// * `config_setting_units` - map with unit name and respective regular expression to evaluate the datatype
///
pub fn validate_config_options(
    config_options: &HashMap<ConfigName, ConfigOption>,
    config_setting_units: &HashMap<String, Regex>,
) -> ValidationResult<()> {
    for (name, option) in config_options {
        let as_of_version = Version::parse(&option.as_of_version)?;

        // 1) check for default values
        if let Some(values) = &option.default_values {
            // 1.1) check if a provided default version matches as_of_version
            util::get_option_value_for_version(name, values, &as_of_version)?;

            for val in values {
                // 1.2) check if default matches the allowed values
                check_allowed_values(name, &val.value, &option.allowed_values)?;
                // 1.3) check if default values match datatype (min, max, unit...)
                check_datatype(config_setting_units, name, &val.value, &option.datatype)?
            }
        }

        // 2) check for recommended values
        if let Some(values) = &option.recommended_values {
            // 2.1) check if a provided recommended version matches as_of_version
            util::get_option_value_for_version(name, values, &as_of_version)?;

            for val in values {
                // 2.2) check if recommended matches the allowed values
                check_allowed_values(name, &val.value, &option.allowed_values)?;
                // 2.3) check if recommended values match datatype (min, max, unit...)
                check_datatype(config_setting_units, name, &val.value, &option.datatype)?
            }
        }

        // prepare "user" data
        let mut user_data = HashMap::new();
        if let Some(dependencies) = &option.depends_on {
            for dependency in dependencies {
                for dep_name in &dependency.option_names {
                    if let Some(dependency_option) = config_options.get(dep_name) {
                        if let Some(dependency_option_recommended) =
                            &dependency_option.recommended_values
                        {
                            let filtered_value = util::get_option_value_for_version(
                                &dep_name,
                                dependency_option_recommended,
                                &as_of_version,
                            )?;

                            user_data.insert(dep_name.name.clone(), filtered_value.value.clone());
                        }
                    } else {
                        return Err(Error::ConfigDependencyMissing {
                            option_name: name.clone(),
                            dependency: dependency.option_names.clone(),
                        });
                    }
                }
            }
        }
        // 3) check if dependency values are available and the recommended value matches the required one
        check_dependencies(name, option, &user_data)?;

        // 4) check if role available
        if option.roles.is_none() {
            return Err(Error::ConfigOptionRoleNotProvided { name: name.clone() });
        }
    }

    Ok(())
}

/// Check if the final used value corresponds to e.g. recommended or default values
///
/// # Arguments
///
/// * `option_name` - name of the config option (config property or environmental variable)
/// * `option_value` - the final value used
/// * `option_values` - possible option names e.g. default or recommended values
/// * `product_version` - the provided product version
///
fn check_option_value_used(
    option_name: &ConfigName,
    option_value: &str,
    option_values: &Option<Vec<OptionValue>>,
    product_version: &Version,
) -> ValidationResult<bool> {
    if let Some(values) = option_values {
        let val = util::get_option_value_for_version(option_name, values, product_version)?;
        if val.value == option_value {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Check if config option role is available
///
/// # Arguments
///
/// * `option_name` - name of the config option
/// * `option_config_roles` - config roles provided in the option definition
/// * `config_role` - config role provided by the user
///
fn check_role(
    option_name: &ConfigName,
    option_config_roles: &Option<Vec<Role>>,
    config_role: Option<&str>,
) -> ValidationResult<()> {
    if option_config_roles.is_none() {
        return Err(Error::ConfigOptionRoleNotProvided {
            name: option_name.clone(),
        });
    }

    if config_role.is_none() {
        return Err(Error::ConfigOptionRoleNotProvidedByUser {
            name: option_name.clone(),
        });
    }

    if let (Some(roles), Some(user_role)) = (option_config_roles, config_role) {
        for role in roles {
            if role.name == user_role {
                return Ok(());
            }
        }
    }

    Err(Error::ConfigOptionRoleNotFound {
        name: option_name.clone(),
        role: config_role.unwrap().to_string(),
    })
}

/// Check if config option version is supported or deprecated regarding the product version
///
/// # Arguments
///
/// * `option_name` - name of the config option
/// * `product_version` - the current product version
/// * `option_version` - as of version of the provided config option
/// * `deprecated_since` - version from which point onwards the option is deprecated
///
fn check_version_supported_or_deprecated(
    option_name: &ConfigName,
    product_version: &Version,
    option_version: &str,
    deprecated_since: &Option<String>,
) -> ValidationResult<()> {
    let option_version = Version::parse(option_version)?;

    // compare version of the config option and product / controller version
    if option_version > *product_version {
        return Err(Error::VersionNotSupported {
            option_name: option_name.clone(),
            product_version: product_version.to_string(),
            required_version: option_version.to_string(),
        });
    }

    // check if requested config option is deprecated
    if let Some(deprecated) = deprecated_since {
        let deprecated_since_version = Version::parse(deprecated.as_ref())?;

        if deprecated_since_version <= *product_version {
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
/// TODO: add dependency automatically if missing?
///
/// # Arguments
///
/// * `option_name` - name of the current option
/// * `config_options` - map with (defined) config option names and the respective config_option
/// * `user_options` - map with config option name and potential value provided by user
///
fn check_dependencies(
    option_name: &ConfigName,
    config_option: &ConfigOption,
    user_options: &HashMap<String, String>,
) -> ValidationResult<()> {
    // check if config option has dependencies
    let config_option_dependencies = match &config_option.depends_on {
        None => return Ok(()),
        Some(dependencies) => dependencies,
    };

    // for each dependency, check if user_options contains the config option and the correct value
    for config_option_dependency in config_option_dependencies {
        // check if we find any matches, otherwise return error after the loop
        let mut found_match = false;
        // for each option name provided within the dependency
        for dependency_option_name in &config_option_dependency.option_names {
            if !user_options.contains_key(&dependency_option_name.name) {
                continue;
            }

            found_match = true;

            match (
                user_options.get(&dependency_option_name.name),
                &config_option_dependency.value,
            ) {
                // no user value, no config value -> ok
                (None, None) => continue,
                // no user value but config value required -> error
                (None, Some(config_value)) => {
                    return Err(Error::ConfigDependencyUserValueMissing {
                        option_name: option_name.clone(),
                        dependency: dependency_option_name.name.clone(),
                        required_value: config_value.clone(),
                    })
                }
                // user value but no config value required -> error
                (Some(user_value), None) => {
                    return Err(Error::ConfigDependencyUserValueNotRequired {
                        option_name: option_name.clone(),
                        dependency: dependency_option_name.name.clone(),
                        user_value: user_value.clone(),
                    })
                }
                // both values available -> check if match
                (Some(user_value), Some(config_value)) => {
                    if user_value != config_value {
                        return Err(Error::ConfigDependencyValueInvalid {
                            option_name: option_name.clone(),
                            dependency: dependency_option_name.name.clone(),
                            user_value: user_value.clone(),
                            required_value: config_value.clone(),
                        });
                    }
                }
            }
        }

        if !found_match {
            // TODO: Error or just add the correct dependency?
            return Err(Error::ConfigDependencyMissing {
                option_name: option_name.clone(),
                dependency: config_option_dependency.option_names.clone(),
            });
        }
    }

    Ok(())
}

/// Check if option value fits the provided datatype
/// # Arguments
///
/// * `config_setting_units` - map with unit name and respective regular expression to evaluate the datatype
/// * `option_name` - name of the config option (config property or environmental variable)
/// * `option_value` - config option value to be validated
/// * `datatype` - option datatype containing min/max bounds, units etc.
///
fn check_datatype(
    config_setting_units: &HashMap<String, Regex>,
    option_name: &ConfigName,
    option_value: &str,
    datatype: &Datatype,
) -> ValidationResult<()> {
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
    option_name: &ConfigName,
    option_value: &str,
    allowed_values: &Option<Vec<String>>,
) -> ValidationResult<()> {
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
    option_name: &ConfigName,
    option_value: &str,
    min: &Option<String>,
    max: &Option<String>,
) -> ValidationResult<T>
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
/// * `config_setting_units` - map with unit name and respective regular expression to evaluate the datatype
/// * `option_name` - name of the config option (config property or environmental variable)
/// * `option_value` - config option value to be validated
/// * `min` - minimum value specified in config_option.data_format.min
/// * `max` - maximum value specified in config_option.data_format.max
/// * `unit` - provided unit to get the regular expression to parse the option_value
///
fn check_datatype_string(
    config_setting_units: &HashMap<String, Regex>,
    option_name: &ConfigName,
    option_value: &str,
    min: &Option<String>,
    max: &Option<String>,
    unit: &Option<String>,
) -> ValidationResult<()> {
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
/// * `option_name` - name of the config option (config property or environmental variable)
/// * `value` - value to be validated
/// * `bound` - upper/lower bound
/// * `check_out_of_bound` - the method to check against the bound
///
fn check_bound<T>(
    option_name: &ConfigName,
    value: T,
    bound: &Option<String>,
    check_out_of_bound: fn(T, T) -> bool,
) -> ValidationResult<T>
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
///
/// # Arguments
///
/// * `option_name` - name of the config option (config property or environmental variable)
/// * `to_parse` - value to be parsed into a certain T
///
fn parse<T: FromStr>(option_name: &ConfigName, to_parse: &str) -> Result<T, Error> {
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
    macro_rules! hashmap {
        ($( $key: expr => $val: expr ),*) => {{
             let mut map = ::std::collections::HashMap::new();
             $( map.insert($key, $val); )*
             map
        }}
    }

    use crate::error::Error;
    use crate::reader::ConfigJsonReader;
    use crate::types::{ConfigKind, ConfigName, Datatype, Role};
    use crate::validation::{
        check_allowed_values, check_datatype, check_dependencies, check_role,
        check_version_supported_or_deprecated,
    };
    use crate::ProductConfig;
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

    fn get_conf_option_name(name: &str, file: &str) -> ConfigName {
        ConfigName {
            name: name.to_string(),
            kind: ConfigKind::Conf(file.to_string()),
        }
    }

    fn get_product_config() -> ProductConfig {
        ProductConfig::new(ConfigJsonReader::new("data/test_config.json")).unwrap()
    }

    #[rstest(
        option_name,
        product_version,
        option_version,
        deprecated_since,
        expected,
        case(get_conf_option_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE), V_1_0_0, V_0_5_0, None, Ok(())),
        case(get_conf_option_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE), V_0_1_0, V_1_0_0, Some(V_0_5_0.to_string()),
            Err(Error::VersionNotSupported { option_name: option_name.clone(), product_version: V_0_1_0.to_string(), required_version: V_1_0_0.to_string() })),
        case(get_conf_option_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE), V_1_5_0, V_0_5_0, Some(V_1_0_0.to_string()),
            Err(Error::VersionDeprecated { option_name: option_name.clone(), product_version: V_1_5_0.to_string(), deprecated_version: V_1_0_0.to_string() })),
        ::trace
    )]
    fn test_check_version_supported_or_deprecated(
        option_name: ConfigName,
        product_version: &str,
        option_version: &str,
        deprecated_since: Option<String>,
        expected: Result<(), Error>,
    ) {
        let result = check_version_supported_or_deprecated(
            &option_name,
            &Version::parse(product_version).unwrap(),
            option_version,
            &deprecated_since,
        );

        assert_eq!(result, expected)
    }

    const ROLE_1: &str = "role_1";
    const ROLE_2: &str = "role_2";

    #[rstest(
        option_name,
        role,
        expected,
        case(
            &get_conf_option_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE),
            Some(ROLE_1),
            Ok(())
        ),
        case(
            &get_conf_option_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE),
            Some(ROLE_2),
            Ok(())
        ),
        case(
            &get_conf_option_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE),
            None,
            Err(Error::ConfigOptionRoleNotProvidedByUser { name: get_conf_option_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE) })
        ),
        ::trace
    )]
    fn test_check_role(option_name: &ConfigName, role: Option<&str>, expected: Result<(), Error>) {
        let option_config_roles = Some(vec![
            Role {
                name: ROLE_1.to_string(),
                required: true,
            },
            Role {
                name: ROLE_2.to_string(),
                required: false,
            },
        ]);

        let result = check_role(option_name, &option_config_roles, role);

        assert_eq!(result, expected)
    }

    #[rstest(
    option_name,
    user_options,
    expected,
    case(
        &get_conf_option_name(ENV_SSL_CERTIFICATE_PATH, CONFIG_FILE),
        hashmap!{
            ENV_SSL_CERTIFICATE_PATH.to_string() => "some/path/to/certificate".to_string()
        },
        Err(Error::ConfigDependencyMissing {
            option_name: get_conf_option_name(ENV_SSL_CERTIFICATE_PATH, CONFIG_FILE),
            dependency: vec![
                ConfigName { name: ENV_SSL_ENABLED.to_string(), kind: ConfigKind::Conf(CONFIG_FILE.to_string()) },
                ConfigName { name: CONF_SSL_ENABLED.to_string(), kind: ConfigKind::Conf(CONFIG_FILE_2.to_string()) }
            ]
        })
    ),
    case(
        &get_conf_option_name(ENV_SSL_CERTIFICATE_PATH, CONFIG_FILE),
        hashmap!{
            ENV_SSL_CERTIFICATE_PATH.to_string() => "some/path/to/certificate".to_string(),
            ENV_SSL_ENABLED.to_string() => "false".to_string()
        },
        Err(Error::ConfigDependencyValueInvalid {
            option_name: get_conf_option_name(ENV_SSL_CERTIFICATE_PATH, CONFIG_FILE),
            dependency: "ENV_SSL_ENABLED".to_string(),
            user_value: "false".to_string(),
            required_value: "true".to_string()
        })
    ),
    case(
        &get_conf_option_name(ENV_SSL_CERTIFICATE_PATH, CONFIG_FILE),
        hashmap!{
            ENV_SSL_CERTIFICATE_PATH.to_string() => "some/path/to/certificate".to_string(),
            ENV_SSL_ENABLED.to_string() => "true".to_string()
        },
        Ok(())
    ),
    ::trace
    )]
    fn test_check_dependencies(
        option_name: &ConfigName,
        user_options: HashMap<String, String>,
        expected: Result<(), Error>,
    ) {
        let product_config = get_product_config();
        let config_option = product_config.config_options.get(&option_name).unwrap();

        let result = check_dependencies(&option_name, config_option, &user_options);

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

    #[rstest(
        option_name,
        option_value,
        datatype,
        expected,
        case(
            &get_conf_option_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE),
            PORT_CORRECT,
            &Datatype::Integer{ min: Some(MIN_PORT.to_string()), max: Some(MAX_PORT.to_string()), unit: Some("port".to_string()), accepted_units: None, default_unit:None },
            Ok(())
        ),
        case(
            &get_conf_option_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE),
            PORT_BAD_DATATYPE,
            &Datatype::Integer{ min: Some(MIN_PORT.to_string()), max: Some(MAX_PORT.to_string()), unit: Some("port".to_string()), accepted_units: None, default_unit:None },
            Err(Error::DatatypeNotMatching { option_name: get_conf_option_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE), value: PORT_BAD_DATATYPE.to_string(), datatype: "i64".to_string() })
        ),
        case(
            &get_conf_option_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE),
            PORT_OUT_OF_BOUNDS,
            &Datatype::Integer{ min: Some(MIN_PORT.to_string()), max: Some(MAX_PORT.to_string()), unit: Some("port".to_string()), accepted_units: None, default_unit:None },
            Err(Error::ConfigValueOutOfBounds { option_name: get_conf_option_name(ENV_INTEGER_PORT_MIN_MAX, CONFIG_FILE), received: PORT_OUT_OF_BOUNDS.to_string(), expected: MAX_PORT.to_string() })
        ),
        case(
            &get_conf_option_name(ENV_PROPERTY_STRING_MEMORY, CONFIG_FILE),
            MEMORY_CORRECT_MB,
            &Datatype::String{ min: None, max: None, unit: Some("memory".to_string()), accepted_units: None, default_unit:None },
            Ok(())
        ),
        case(
            &get_conf_option_name(ENV_PROPERTY_STRING_MEMORY, CONFIG_FILE),
            MEMORY_CORRECT_GB,
            &Datatype::String{ min: None, max: None, unit: Some("memory".to_string()), accepted_units: None, default_unit:None },
            Ok(())
        ),
        case(
            &get_conf_option_name(ENV_PROPERTY_STRING_MEMORY, CONFIG_FILE),
            MEMORY_MISSING_UNIT,
            &Datatype::String{ min: None, max: None, unit: Some("memory".to_string()), accepted_units: None, default_unit:None },
            Err(Error::DatatypeRegexNotMatching { option_name: get_conf_option_name(ENV_PROPERTY_STRING_MEMORY, CONFIG_FILE), value: MEMORY_MISSING_UNIT.to_string() })
        ),
        case(
            &get_conf_option_name(ENV_VAR_FLOAT, CONFIG_FILE),
            FLOAT_CORRECT,
            &Datatype::Float{ min: Some("0.0".to_string()), max: Some("100.0".to_string()), unit: None, accepted_units: None, default_unit:None },
            Ok(())
        ),
        case(
            &get_conf_option_name(ENV_VAR_FLOAT, CONFIG_FILE),
            FLOAT_BAD,
            &Datatype::Float{ min: Some("0.0".to_string()), max: Some("100.0".to_string()), unit: None, accepted_units: None, default_unit:None },
            Err(Error::DatatypeNotMatching { option_name: get_conf_option_name(ENV_VAR_FLOAT, CONFIG_FILE), value: FLOAT_BAD.to_string(), datatype: "f64".to_string() })
        ),
    ::trace
    )]
    fn test_check_datatype(
        option_name: &ConfigName,
        option_value: &str,
        datatype: &Datatype,
        expected: Result<(), Error>,
    ) {
        let config_setting_units = get_product_config().config_setting_units;

        let result = check_datatype(&config_setting_units, option_name, option_value, &datatype);

        assert_eq!(result, expected)
    }

    const ALLOWED_VALUE_1: &str = "allowed_value_1";
    const ALLOWED_VALUE_2: &str = "allowed_value_2";
    const ALLOWED_VALUE_3: &str = "allowed_value_3";
    const NOT_ALLOWED_VALUE: &str = "not_allowed_value";

    #[rstest(
        option_name,
        option_value,
        allowed_values,
        expected,
        case(
            &get_conf_option_name(ENV_ALLOWED_VALUES, CONFIG_FILE),
            ALLOWED_VALUE_1,
            Some(vec![ALLOWED_VALUE_1.to_string(), ALLOWED_VALUE_2.to_string(), ALLOWED_VALUE_3.to_string()]),
            Ok(())
        ),
        case(
            &get_conf_option_name(ENV_ALLOWED_VALUES, CONFIG_FILE),
            NOT_ALLOWED_VALUE,
            Some(vec![ALLOWED_VALUE_1.to_string(), ALLOWED_VALUE_2.to_string(), ALLOWED_VALUE_3.to_string()]),
            Err(Error::ConfigValueNotInAllowedValues {
                option_name: get_conf_option_name(ENV_ALLOWED_VALUES, CONFIG_FILE),
                value: NOT_ALLOWED_VALUE.to_string(),
                allowed_values: vec![ALLOWED_VALUE_1.to_string(), ALLOWED_VALUE_2.to_string(), ALLOWED_VALUE_3.to_string() ]
            })
        ),
    ::trace
    )]
    fn test_check_allowed_values(
        option_name: &ConfigName,
        option_value: &str,
        allowed_values: Option<Vec<String>>,
        expected: Result<(), Error>,
    ) {
        let result = check_allowed_values(option_name, option_value, &allowed_values);

        assert_eq!(result, expected)
    }
}
