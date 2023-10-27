use java_properties::{PropertiesError, PropertiesWriter};
use snafu::{ResultExt, Snafu};
use std::io::Write;
use xml::escape::escape_str_attribute;

#[derive(Debug, Snafu)]
pub enum PropertiesWriterError {
    #[snafu(display("failed to create properties file"))]
    PropertiesError { source: PropertiesError },

    #[snafu(display("failed to convert properties file byte array to UTF-8"))]
    FromUtf8Error { source: std::string::FromUtf8Error },
}

/// Creates a common Java properties file string in the format:
/// property_1=value_1\n
/// property_2=value_2\n
///
/// The behavior is based on <https://docs.oracle.com/javase/7/docs/api/java/util/Properties.html>
/// and is adapted to "java.util.Properties" if ambiguous or incomplete.
pub fn to_java_properties_string<'a, T>(properties: T) -> Result<String, PropertiesWriterError>
where
    T: Iterator<Item = (&'a String, &'a Option<String>)>,
{
    let mut output = Vec::new();
    write_java_properties(&mut output, properties)?;
    String::from_utf8(output).context(FromUtf8Snafu)
}

/// Generic method to write java properties
/// Accepts HashMap<String, Option<String>> or BTreeMap<String, Option<String>>.
/// The map is written as follows (where key=String and val=Option<String>)
/// val = None          -> key=
/// val = Some("")      -> key=
/// val = Some("foo")   -> key=abc
pub fn write_java_properties<'a, W, T>(
    writer: W,
    properties: T,
) -> Result<(), PropertiesWriterError>
where
    W: Write,
    T: Iterator<Item = (&'a String, &'a Option<String>)>,
{
    let mut writer = PropertiesWriter::new(writer);
    for (k, v) in properties {
        let property_value = v.as_deref().unwrap_or_default();
        writer.write(k, property_value).context(PropertiesSnafu)?;
    }

    writer.flush().context(PropertiesSnafu)?;
    Ok(())
}

/// Converts properties into a Hadoop configuration XML snippet.
///
/// This is missing the wrapping `<configuration>...</configuration>` elements so it can be composed.
/// Elements for which the value is `None` will be ignored.
/// Empty values (i.e. `""`) will be returned though.
/// This method will properly escape all keys and values to be safe to use in XML.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use product_config::writer::to_hadoop_xml_snippet;
/// let mut map = HashMap::new();
/// map.insert("foo".to_string(), Some("bar".to_string()));
/// map.insert("baz".to_string(), Some("foo".to_string()));
/// map.insert("bar".to_string(), None);
/// let result = to_hadoop_xml_snippet(map.iter());
/// ```
pub fn to_hadoop_xml_snippet<'a, T>(properties: T) -> String
where
    T: Iterator<Item = (&'a String, &'a Option<String>)>,
{
    let mut result = String::new();
    for (k, v) in properties {
        let escaped_value = match v {
            Some(value) => escape_str_attribute(value),
            None => continue,
        };
        let escaped_key = escape_str_attribute(k);
        result.push_str(&format!(
            "  <property>\n    <name>{}</name>\n    <value>{}</value>\n  </property>\n",
            escaped_key, escaped_value
        ));
    }
    result
}

/// Converts properties into a Hadoop configuration XML.
///
/// This includes the wrapping `<configuration>...</configuration>` elements so it cannot be composed.
/// If you're looking for a composable version look at [`crate::writer::to_hadoop_xml_snippet`].
/// Elements for which the value is `None` will be ignored.
/// Empty values (i.e. `""`) will be returned though.
/// This method will properly escape all keys and values to be safe to use in XML.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use product_config::writer::to_hadoop_xml;
/// let mut map = HashMap::new();
/// map.insert("foo".to_string(), Some("bar".to_string()));
/// map.insert("baz".to_string(), Some("foo".to_string()));
/// map.insert("bar".to_string(), None);
/// let result = to_hadoop_xml(map.iter());
/// ```
pub fn to_hadoop_xml<'a, T>(properties: T) -> String
where
    T: Iterator<Item = (&'a String, &'a Option<String>)>,
{
    wrap_hadoop_xml_snippet(to_hadoop_xml_snippet(properties))
}

/// This wraps a XML snippet with the required XML elements to make a Hadoop XML file.
///
/// See [`to_hadoop_xml`] and [`to_hadoop_xml_snippet`].
pub fn wrap_hadoop_xml_snippet<T: AsRef<str>>(snippet: T) -> String {
    format!(
        "<?xml version=\"1.0\"?>\n<configuration>\n{}</configuration>",
        snippet.as_ref()
    )
}

#[cfg(test)]
mod tests {
    use crate::writer::{
        to_hadoop_xml, to_hadoop_xml_snippet, to_java_properties_string, write_java_properties,
        PropertiesWriterError,
    };
    use std::collections::{BTreeMap, HashMap};

    const PROPERTY_1: &str = "property";
    const PROPERTY_2: &str = "property2";
    const VALUE_OK: &str = "ab&c";
    const VALUE_OK_2: &str = "some_text!()";
    const VALUE_OK_2_ESCAPED: &str = "some_text\\!()";
    const VALUE_URL: &str = "file://this/location/file.abc";
    const VALUE_URL_ESCAPED: &str = "file\\://this/location/file.abc";
    const UTF8_ERROR: &str = "æææ";

    #[test]
    fn test_xml_snippet() {
        let mut map = HashMap::new();
        map.insert(PROPERTY_1.to_string(), Some(VALUE_OK.to_string()));
        map.insert(PROPERTY_2.to_string(), Some(VALUE_OK_2.to_string()));
        map.insert("foo".to_string(), None);

        let result = to_hadoop_xml_snippet(map.iter());
        assert!(result.contains("ab&amp;"));
        assert!(!result.contains("foo"));
        assert!(result.contains(PROPERTY_2));
    }

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

        let expected = "empty=\nnone=\nnormal=normal\n";

        let mut output = Vec::new();
        write_java_properties(&mut output, btree_map.iter()).unwrap();

        let result = String::from_utf8(output).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_xml_escape_attributes() {
        // TODO: make rstest and check pc data as well
        let mut data = BTreeMap::new();
        let no_escaping = "file:///foo:bar/foo?bar=123";
        let to_escape = "<abc>";
        let to_escape_expected = "&lt;abc&gt;";

        data.insert("not_escaped".to_string(), Some(no_escaping.to_string()));
        data.insert("to_escaped".to_string(), Some(to_escape.to_string()));

        let result = to_hadoop_xml(data.iter());

        assert!(result.contains(no_escaping));
        assert!(result.contains(to_escape_expected));
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
