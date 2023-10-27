//! Writer for Flask App configurations.
//!
//! Applications based on the [Flask App Builder], like [Apache Superset] and [Apache Airflow], use
//! configuration files written in Python. This writer creates such configuration files from
//! product configurations. It is not a general Python code generator. It only covers the parts
//! which are commonly used in these configuration files, i.e. assignments of some primitive data
//! types and expressions at the top-level.
//!
//! Primitive data types are escaped accordingly. Python expressions are written as is to the
//! configuration file. Invalid expressions produce invalid configuration files. Expressions are
//! useful to statically set options within the operator but these options should not be exposed to
//! the users. Nevertheless, users can override (non-exposed) options which are treated as plain
//! expressions. So users must take care when doing this.
//!
//! [Flask App Builder]: http://flaskappbuilder.pythonanywhere.com/
//! [Apache Superset]: https://superset.apache.org/
//! [Apache Airflow]: https://airflow.apache.org/
//!
//! # Example
//!
//! ```
//! use std::{collections::BTreeMap, str::FromStr};
//! use product_config::flask_app_config_writer::{
//!     self,
//!     FlaskAppConfigOptions,
//!     FlaskAppConfigWriterError,
//!     PythonType,
//! };
//!
//! // Enumeration of all supported options
//! enum ApplicationOptions {
//!     AuthType,
//!     Profiling,
//!     SecretKey,
//! }
//!
//! // Mapping from strings to options which is used to map the keys in the product configuration
//! // to the known options.
//! // This mapping can also be auto-generated with [strum](https://crates.io/crates/strum).
//! impl FromStr for ApplicationOptions {
//!     type Err = &'static str;
//!
//!     fn from_str(s: &str) -> Result<Self, Self::Err> {
//!         match s {
//!             "AUTH_TYPE" => Ok(ApplicationOptions::AuthType),
//!             "PROFILING" => Ok(ApplicationOptions::Profiling),
//!             "SECRET_KEY" => Ok(ApplicationOptions::SecretKey),
//!             _ => Err("unknown option"),
//!         }
//!     }
//! }
//!
//! // Mapping from options to Python types which is used to generate the values of the assignments.
//! impl FlaskAppConfigOptions for ApplicationOptions {
//!     fn python_type(&self) -> PythonType {
//!         match self {
//!             ApplicationOptions::AuthType => PythonType::Expression,
//!             ApplicationOptions::Profiling => PythonType::BoolLiteral,
//!             ApplicationOptions::SecretKey => PythonType::Expression,
//!         }
//!     }
//! }
//!
//! fn build_config_file(
//!     product_config: &BTreeMap<String, String>,
//! ) -> Result<String, FlaskAppConfigWriterError> {
//!     let imports = ["import os"];
//!
//!     let mut extended_config = product_config.clone();
//!     extended_config.insert("SECRET_KEY".into(), "os.environ.get(\"SECRET_KEY\")".into());
//!
//!     let mut config_file = Vec::new();
//!     flask_app_config_writer::write::<ApplicationOptions, _, _>(
//!         &mut config_file,
//!         extended_config.iter(),
//!         &imports,
//!     )?;
//!
//!     Ok(String::from_utf8(config_file).unwrap())
//! }
//!
//! let product_config = [
//!     ("AUTH_TYPE".into(), "AUTH_DB".into()),
//!     ("PROFILING".into(), "false".into()),
//!     // Config overrides are always of type `PythonType::Expression`.
//!     ("DEBUG".into(), "True".into()),
//! ]
//! .into();
//!
//! let config_file = build_config_file(&product_config).unwrap();
//!
//! assert_eq!(
//!     r#"import os
//!
//! AUTH_TYPE = AUTH_DB
//! DEBUG = True
//! PROFILING = False
//! SECRET_KEY = os.environ.get("SECRET_KEY")
//! "#,
//!     config_file
//! );
//!
//! ```

use std::{
    io::{self, Write},
    num::ParseIntError,
    str::{FromStr, ParseBoolError},
};

use snafu::{ResultExt, Snafu};

/// Errors which can occur when using this module
#[derive(Debug, Snafu)]
pub enum FlaskAppConfigWriterError {
    #[snafu(display("failed to convert '{value}' into a identifier"))]
    ConvertIdentifierError { value: String },

    #[snafu(display("failed to convert '{value}' into a boolean literal"))]
    ConvertBoolLiteralError {
        value: String,
        source: ParseBoolError,
    },

    #[snafu(display("failed to convert '{value}' into an integer literal"))]
    ConvertIntLiteralError {
        value: String,
        source: ParseIntError,
    },

    #[snafu(display("failed to convert '{value}' into an ASCII string literal"))]
    ConvertStringLiteralError { value: String },

    #[snafu(display("failed to convert '{value}' into a Python expression"))]
    ConvertExpressionError { value: String },

    #[snafu(display("Configuration cannot be written."))]
    WriteConfigError { source: io::Error },
}

/// Mapping from configuration options to Python types.
pub trait FlaskAppConfigOptions {
    fn python_type(&self) -> PythonType;
}

/// All supported Python types
pub enum PythonType {
    /// Python identifier
    Identifier,
    /// Boolean literal
    BoolLiteral,
    /// Integer literal
    IntLiteral,
    /// ASCII string literal
    StringLiteral,
    /// Python expression
    Expression,
}

impl PythonType {
    /// Converts the given string to Python.
    fn convert_to_python(&self, value: &str) -> Result<String, FlaskAppConfigWriterError> {
        let convert = match self {
            PythonType::Identifier => PythonType::convert_to_python_identifier,
            PythonType::BoolLiteral => PythonType::convert_to_python_bool_literal,
            PythonType::IntLiteral => PythonType::convert_to_python_int_literal,
            PythonType::StringLiteral => PythonType::convert_to_python_string_literal,
            PythonType::Expression => PythonType::convert_to_python_expression,
        };

        convert(value)
    }

    fn convert_to_python_identifier(value: &str) -> Result<String, FlaskAppConfigWriterError> {
        if value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && value
                .chars()
                .next()
                .filter(|c| !c.is_ascii_digit())
                .is_some()
        {
            Ok(value.to_string())
        } else {
            ConvertIdentifierSnafu { value }.fail()
        }
    }

    fn convert_to_python_bool_literal(value: &str) -> Result<String, FlaskAppConfigWriterError> {
        value
            .parse::<bool>()
            .map(|b| if b { "True".into() } else { "False".into() })
            .context(ConvertBoolLiteralSnafu { value })
    }

    fn convert_to_python_int_literal(value: &str) -> Result<String, FlaskAppConfigWriterError> {
        value
            .parse::<i64>()
            .map(|i| i.to_string())
            .context(ConvertIntLiteralSnafu { value })
    }

    fn convert_to_python_string_literal(value: &str) -> Result<String, FlaskAppConfigWriterError> {
        if value.is_ascii() {
            Ok(format!("\"{}\"", value.escape_default()))
        } else {
            ConvertStringLiteralSnafu { value }.fail()
        }
    }

    fn convert_to_python_expression(value: &str) -> Result<String, FlaskAppConfigWriterError> {
        if !value.trim().is_empty() {
            Ok(value.to_string())
        } else {
            ConvertExpressionSnafu { value }.fail()
        }
    }
}

/// Writes a configuration file according to the given `FlaskAppConfigOptions` type.
pub fn write<'a, O, P, W>(
    writer: &mut W,
    properties: P,
    imports: &[&str],
) -> Result<(), FlaskAppConfigWriterError>
where
    O: FlaskAppConfigOptions + FromStr,
    P: Iterator<Item = (&'a String, &'a String)>,
    W: Write,
{
    for import in imports {
        writeln!(writer, "{import}").context(WriteConfigSnafu)?;
    }

    writeln!(writer).context(WriteConfigSnafu)?;

    for (name, value) in properties {
        let variable = PythonType::Identifier.convert_to_python(name)?;

        // If an option cannot be mapped to a Python type then it is a config override and treated
        // as Python expression.
        let content = O::from_str(name)
            .map(|option| option.python_type())
            .unwrap_or(PythonType::Expression)
            .convert_to_python(value)?;

        writeln!(writer, "{variable} = {content}").context(WriteConfigSnafu)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{write, FlaskAppConfigOptions, FlaskAppConfigWriterError, PythonType};
    use rstest::*;
    use std::{
        collections::BTreeMap,
        str::{from_utf8, FromStr},
    };

    #[rstest]
    #[case::valid_identifiers_are_converted_to_python(
        PythonType::Identifier, &[
            ("_", "_"),
            ("a", "a"),
            ("A", "A"),
            ("__", "__"),
            ("_a", "_a"),
            ("_A", "_A"),
            ("_0", "_0"),
            ("SECRET_KEY", "SECRET_KEY"),
        ]
    )]
    #[case::valid_booleans_are_converted_to_python(
        PythonType::BoolLiteral, &[
            ("False", "false"),
            ("True", "true"),
        ]
    )]
    #[case::valid_integers_are_converted_to_python(
        PythonType::IntLiteral, &[
            ("-9223372036854775808", "-9223372036854775808"),
            ("0", "0"),
            ("9223372036854775807", "9223372036854775807"),
        ]
    )]
    #[case::valid_strings_are_converted_to_python(
        PythonType::StringLiteral, &[
            (r#""""#, ""),
            (r#"" ~""#, " ~"),
            (r#""\t\r\n\'\"\\""#, "\t\r\n'\"\\"),
        ]
    )]
    #[case::valid_expressions_are_converted_to_python(
        PythonType::Expression, &[
            ("os.environ[\"HOME\"]", "os.environ[\"HOME\"]"),
        ]
    )]
    fn valid_values_are_converted_to_python(
        #[case] python_type: PythonType,
        #[case] values: &[(&str, &str)],
    ) -> Result<(), FlaskAppConfigWriterError> {
        for (expected, input) in values {
            assert_eq!(*expected, python_type.convert_to_python(input)?);
        }

        Ok(())
    }

    #[rstest]
    #[case::invalid_identifiers_are_not_converted_to_python(
        PythonType::Identifier, &[
            "", "0", "-", "\n", "_-", "_\n",
        ]
    )]
    #[case::invalid_booleans_are_not_converted_to_python(
        PythonType::BoolLiteral, &[
            "", "False", "True", "0", "1",
        ]
    )]
    #[case::invalid_integers_are_not_converted_to_python(
        PythonType::IntLiteral, &[
            "", "a", "0x10", "inf",
        ]
    )]
    #[case::invalid_strings_are_not_converted_to_python(
        PythonType::StringLiteral, &[
            "ä", "❤"
        ]
    )]
    #[case::invalid_expressions_are_not_converted_to_python(
        PythonType::Expression, &[
            ""
        ]
    )]
    fn invalid_values_are_converted_to_python(
        #[case] python_type: PythonType,
        #[case] values: &[&str],
    ) {
        for input in values {
            assert!(python_type.convert_to_python(input).is_err());
        }
    }

    #[test]
    fn valid_options_are_written_into_a_configuration() -> Result<(), FlaskAppConfigWriterError> {
        #[allow(clippy::enum_variant_names)]
        enum Options {
            BoolOption,
            IntOption,
            StringOption,
            ExpressionOption,
            _UnusedOption,
        }

        impl FromStr for Options {
            type Err = &'static str;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    "BOOL_OPTION" => Ok(Options::BoolOption),
                    "INT_OPTION" => Ok(Options::IntOption),
                    "STRING_OPTION" => Ok(Options::StringOption),
                    "EXPRESSION_OPTION" => Ok(Options::ExpressionOption),
                    _ => Err("unknown option"),
                }
            }
        }

        impl FlaskAppConfigOptions for Options {
            fn python_type(&self) -> PythonType {
                match self {
                    Options::BoolOption => PythonType::BoolLiteral,
                    Options::IntOption => PythonType::IntLiteral,
                    Options::StringOption => PythonType::StringLiteral,
                    Options::ExpressionOption => PythonType::Expression,
                    Options::_UnusedOption => PythonType::Expression,
                }
            }
        }

        let config: BTreeMap<_, _> = [
            ("BOOL_OPTION", "true"),
            ("INT_OPTION", "0"),
            ("STRING_OPTION", ""),
            ("EXPRESSION_OPTION", "{ \"key\": \"value\" }"),
            ("OVERRIDDEN_OPTION", "None"),
        ]
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .into();

        let imports = ["import module", "from module import member"];

        let mut config_file = Vec::new();
        write::<Options, _, _>(&mut config_file, config.iter(), &imports)?;

        assert_eq!(
            r#"import module
from module import member

BOOL_OPTION = True
EXPRESSION_OPTION = { "key": "value" }
INT_OPTION = 0
OVERRIDDEN_OPTION = None
STRING_OPTION = ""
"#,
            from_utf8(&config_file).unwrap()
        );

        Ok(())
    }
}
