use java_properties::write;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum PropertiesWriterError {
    #[error("Error creating properties file: {0}")]
    PropertiesError(String),

    #[error("Error converting properties file byte array to UTF-8")]
    FromUtf8Error(#[from] std::string::FromUtf8Error),
}

/// Creates a common Java properties file string in the format:
/// property_1=value_1\n
/// property_2=value_2\n
///
/// The behavior is based on https://docs.oracle.com/javase/7/docs/api/java/util/Properties.html
/// and is adapted to "java.util.Properties" if ambiguous or incomplete.
pub fn create_java_properties_file(
    properties: &HashMap<String, String>,
) -> Result<String, PropertiesWriterError> {
    let mut output = Vec::new();
    write(&mut output, &properties)
        .map_err(|err| PropertiesWriterError::PropertiesError(err.to_string()))?;
    Ok(String::from_utf8(output)?)
}

#[cfg(test)]
mod tests {
    use crate::writer::{create_java_properties_file, PropertiesWriterError};
    use std::collections::HashMap;

    const PROPERTY_1: &str = "property";
    const PROPERTY_2: &str = "property2";
    const VALUE_OK: &str = "abc";
    const VALUE_OK_2: &str = "some_text!()";
    const VALUE_OK_2_ESCAPED: &str = "some_text\\!()";
    const VALUE_URL: &str = "file://this/location/file.abc";
    const VALUE_URL_ESCAPED: &str = "file\\://this/location/file.abc";
    const UTF8_ERROR: &str = "æææ";

    #[test]
    fn test_writer_ok() -> Result<(), PropertiesWriterError> {
        let mut map = HashMap::new();
        map.insert(PROPERTY_1.to_string(), VALUE_OK.to_string());
        map.insert(PROPERTY_2.to_string(), VALUE_OK_2.to_string());

        let result = create_java_properties_file(&map)?;

        map.insert(PROPERTY_2.to_string(), VALUE_OK_2_ESCAPED.to_string());
        assert_eq!(result, calculate_result(&map));
        Ok(())
    }

    #[test]
    fn test_writer_escape() -> Result<(), PropertiesWriterError> {
        let mut map = HashMap::new();
        map.insert(PROPERTY_1.to_string(), VALUE_URL.to_string());

        let result = create_java_properties_file(&map)?;

        map.insert(PROPERTY_1.to_string(), VALUE_URL_ESCAPED.to_string());
        assert_eq!(result, calculate_result(&map));
        Ok(())
    }

    #[test]
    fn test_writer_no_utf8() {
        let mut map = HashMap::new();
        map.insert(PROPERTY_1.to_string(), UTF8_ERROR.to_string());

        let result = create_java_properties_file(&map);
        assert!(result.is_err());
    }

    fn calculate_result(properties: &HashMap<String, String>) -> String {
        let mut result = String::new();

        for (key, value) in properties {
            result.push_str(&format!("{}={}\n", key, value));
        }

        result
    }
}
