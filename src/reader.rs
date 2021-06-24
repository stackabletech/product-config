//! A config reader implementation to abstract the source of config data
//!
//! For now only JSON as source is supported.
//!
//! Possible extensions: YAML, CSV, database ...
use crate::error::Error;
use crate::types::{ProductConfigSpecProperties, PropertySpec, Unit};
use crate::ProductConfigSpec;
use regex::Regex;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

/// trait for different config readers for json or yaml
pub trait ConfigReader {
    fn read(&self) -> Result<ProductConfigSpec, Error>;
}

/// specific json config reader struct
pub struct ConfigJsonReader {
    config_spec_path: String,
    property_spec_path: String,
}

#[derive(Deserialize, Debug)]
struct JsonProductConfigSpecProperties {
    pub units: Vec<Unit>,
}

impl ConfigJsonReader {
    pub fn new(config_spec_path: &str, property_spec_path: &str) -> Self {
        ConfigJsonReader {
            config_spec_path: config_spec_path.to_string(),
            property_spec_path: property_spec_path.to_string(),
        }
    }
}

impl ConfigReader for ConfigJsonReader {
    fn read(&self) -> Result<ProductConfigSpec, Error> {
        let config_spec: JsonProductConfigSpecProperties = read_file(&self.config_spec_path)?;
        let property_spec: Vec<PropertySpec> = read_file(&self.property_spec_path)?;
        parse_json_config_spec(&config_spec, &property_spec)
    }
}

fn read_file<T: DeserializeOwned>(path: &str) -> Result<T, Error> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => {
            return Err(Error::FileNotFound {
                file_name: path.to_string(),
            });
        }
    };

    let reader = BufReader::new(file);
    match serde_json::from_reader(reader) {
        Ok(t) => Ok(t),
        Err(err) => Err(Error::FileNotParsable {
            file_name: path.to_string(),
            reason: err.to_string(),
        }),
    }
}

/// Parse the provided config spec. Store the property spec in a hashmap with the property name
/// as key and spec as value. Parse any additional settings like units and the respective regex patterns.
///
/// # Arguments
///
/// * `config_spec` - the config spec provided by the JsonConfigReader
/// * `property_spec` - the property spec provided by the JsonConfigReader
///
fn parse_json_config_spec(
    config_spec: &JsonProductConfigSpecProperties,
    property_spec: &[PropertySpec],
) -> Result<ProductConfigSpec, Error> {
    // pack unit name and compiled regex pattern into map
    let mut config_spec_units = HashMap::new();
    for unit in &config_spec.units {
        let unit_name = if unit.name.is_empty() {
            return Err(Error::ConfigSpecPropertiesNotFound {
                name: "unit".to_string(),
            });
        } else {
            unit.name.clone()
        };

        // no regex or empty regex provided
        let unit_regex = if unit.regex == "".to_string() {
            return Err(Error::EmptyRegexPattern {
                unit: unit.name.clone(),
            });
        } else {
            unit.regex.clone()
        };

        let regex = match Regex::new(unit_regex.as_str()) {
            Ok(regex) => regex,
            Err(_) => {
                return Err(Error::InvalidRegexPattern {
                    unit: unit_name,
                    regex: unit_regex,
                });
            }
        };

        config_spec_units.insert(unit_name, regex);
    }

    // pack properties via name into hashmap for easier access
    let mut parsed_property_spec = HashMap::new();
    for property in property_spec {
        // for every provided property name, write property name and spec into map
        for property_name in &property.property_names {
            parsed_property_spec.insert(property_name.clone(), property.clone());
        }
    }

    Ok(ProductConfigSpec {
        config_spec: ProductConfigSpecProperties {
            units: config_spec_units,
        },
        property_specs: parsed_property_spec,
    })
}
