use java_properties::write;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PropertiesWriterError {
    #[error("Error creating properties file: {0}")]
    PropertiesError(String),

    #[error("Error converting properties file byte array to UTF-8")]
    FromUtf8Error(#[from] std::string::FromUtf8Error),
}

pub fn create_properties_file(
    properties: &HashMap<String, String>,
) -> Result<String, PropertiesWriterError> {
    let mut output = Vec::new();
    write(&mut output, &properties)
        .map_err(|err| PropertiesWriterError::PropertiesError(err.to_string()))?;
    Ok(String::from_utf8(output)?)
}
