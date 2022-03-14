use crate::error::Error;
use crate::types::{Datatype, PropertySpec, Unit};
use std::fmt::Display;
use std::str::FromStr;

pub type ValidationResult<T> = Result<T, Error>;

/// Check if property value is in allowed values
/// # Arguments
///
/// * `property_name` - name of the property
/// * `property_value` - property value to be validated
/// * `allowed_values` - vector of allowed values
///
pub(crate) fn check_allowed_values(
    property_name: &str,
    property_value: &str,
    allowed_values: &Option<Vec<String>>,
) -> ValidationResult<()> {
    if allowed_values.is_some() {
        let allowed_values = allowed_values.clone().unwrap();
        if !allowed_values.is_empty() && !allowed_values.contains(&property_value.to_string()) {
            return Err(Error::PropertyValueNotInAllowedValues {
                property_name: property_name.to_string(),
                value: property_value.to_string(),
                allowed_values,
            });
        }
    }
    Ok(())
}

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
        match unit.regex.is_match(value) {
            Ok(is_match) => {
                if !is_match {
                    return Err(Error::DatatypeRegexNotMatching {
                        property_name: name.to_string(),
                        value: value.to_string(),
                    });
                }
            }
            Err(e) => {
                return Err(Error::RegexNotEvaluatable {
                    property_name: name.to_string(),
                    unit: unit.name.to_string(),
                    regex: unit.regex.to_string(),
                    reason: e.to_string(),
                })
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use rstest::*;

    // TODO: test check_datatype()

    const ALLOWED_VALUE_1: &str = "allowed_value_1";
    const ALLOWED_VALUE_2: &str = "allowed_value_2";
    const ALLOWED_VALUE_3: &str = "allowed_value_3";
    const NOT_ALLOWED_VALUE: &str = "not_allowed_value";

    #[rstest]
    #[case(
        "ENV_ALLOWED_VALUES",
        ALLOWED_VALUE_1,
        Some(vec![ALLOWED_VALUE_1.to_string(), ALLOWED_VALUE_2.to_string(), ALLOWED_VALUE_3.to_string()]),
        Ok(())
    )]
    #[case(
        "ENV_ALLOWED_VALUES",
        NOT_ALLOWED_VALUE,
        Some(vec![ALLOWED_VALUE_1.to_string(), ALLOWED_VALUE_2.to_string(), ALLOWED_VALUE_3.to_string()]),
        Err(Error::PropertyValueNotInAllowedValues {
            property_name: "ENV_ALLOWED_VALUES".to_string(),
            value: NOT_ALLOWED_VALUE.to_string(),
            allowed_values: vec![ALLOWED_VALUE_1.to_string(), ALLOWED_VALUE_2.to_string(), ALLOWED_VALUE_3.to_string() ]
        })
    )]
    fn test_check_allowed_values(
        #[case] property_name: &str,
        #[case] property_value: &str,
        #[case] allowed_values: Option<Vec<String>>,
        #[case] expected: Result<(), Error>,
    ) {
        let result = check_allowed_values(property_name, property_value, &allowed_values);

        assert_eq!(result, expected)
    }
}
