use java_properties::{PropertiesError, PropertiesWriter};
use std::io::Write;
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
pub fn to_java_properties_string<'a, T>(properties: T) -> Result<String, PropertiesWriterError>
where
    T: Iterator<Item = (&'a String, &'a Option<String>)>,
{
    let mut output = Vec::new();
    write_java_properties(&mut output, properties)
        .map_err(|err| PropertiesWriterError::PropertiesError(err.to_string()))?;
    Ok(String::from_utf8(output)?)
}

/// Generic method to write java properties
/// Accepts HashMap<String, Option<String>> or BTreeMap<String, Option<String>>.
/// The map is written as follows (where key=String and val=Option<String>)
/// val = None          -> key=
/// val = Some("")      -> key=""
/// val = Some("foo")   -> key="abc"
pub fn write_java_properties<'a, W, T>(writer: W, properties: T) -> Result<(), PropertiesError>
where
    W: Write,
    T: Iterator<Item = (&'a String, &'a Option<String>)>,
{
    let mut writer = PropertiesWriter::new(writer);
    for (k, v) in properties {
        if let Some(value) = v {
            if value.is_empty() {
                writer.write(&k, "\"\"")?;
            } else {
                writer.write(&k, &value)?;
            }
        } else {
            writer.write(&k, "")?;
        }
    }

    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::writer::{to_java_properties_string, write_java_properties, PropertiesWriterError};
    use std::collections::{BTreeMap, HashMap};

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
        map.insert(PROPERTY_1.to_string(), Some(VALUE_OK.to_string()));
        map.insert(PROPERTY_2.to_string(), Some(VALUE_OK_2.to_string()));

        let result = to_java_properties_string(map.iter())?;

        map.insert(PROPERTY_2.to_string(), Some(VALUE_OK_2_ESCAPED.to_string()));
        assert_eq!(result, calculate_result(map.iter()));
        Ok(())
    }

    #[test]
    fn test_writer_escape() -> Result<(), PropertiesWriterError> {
        let mut map = HashMap::new();
        map.insert(PROPERTY_1.to_string(), Some(VALUE_URL.to_string()));

        let result = to_java_properties_string(map.iter())?;

        map.insert(PROPERTY_1.to_string(), Some(VALUE_URL_ESCAPED.to_string()));
        assert_eq!(result, calculate_result(map.iter()));
        Ok(())
    }

    #[test]
    fn test_writer_no_utf8() {
        let mut map = HashMap::new();
        map.insert(PROPERTY_1.to_string(), Some(UTF8_ERROR.to_string()));

        let result = to_java_properties_string(map.iter());
        assert!(result.is_err());
    }

    #[test]
    fn test_write_java_properties() {
        let mut btree_map = BTreeMap::new();
        btree_map.insert("normal".to_string(), Some("normal".to_string()));
        btree_map.insert("empty".to_string(), Some("".to_string()));
        btree_map.insert("none".to_string(), None);

        let expected = "empty=\"\"\nnone=\nnormal=normal\n";

        let mut output = Vec::new();
        write_java_properties(&mut output, btree_map.iter())
            .map_err(|err| PropertiesWriterError::PropertiesError(err.to_string()))
            .unwrap();

        let result = String::from_utf8(output).unwrap();
        assert_eq!(result, expected);
    }

    fn calculate_result<'a, T>(properties: T) -> String
    where
        T: Iterator<Item = (&'a String, &'a Option<String>)>,
    {
        let mut result = String::new();

        for (k, v) in properties {
            if let Some(value) = v {
                if value.is_empty() {
                    result.push_str(&format!("{}={}\n", &k, "\"\""));
                } else {
                    result.push_str(&format!("{}={}\n", &k, &value));
                }
            } else {
                result.push_str(&format!("{}={}\n", &k, ""));
            }
        }

        result
    }
}
