use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Represents config spec like unit and regex specification
#[derive(Clone, Debug)]
pub(crate) struct ProductConfigSpecProperties {
    pub units: HashMap<String, Regex>,
}

/// Represents one property spec entry for a given property
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialOrd, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PropertySpec {
    pub property_names: Vec<PropertyName>,
    pub datatype: Datatype,
    pub default_values: Option<Vec<PropertyValueSpec>>,
    pub recommended_values: Option<Vec<PropertyValueSpec>>,
    pub allowed_values: Option<Vec<String>>,
    pub as_of_version: String,
    pub deprecated_since: Option<String>,
    pub deprecated_for: Option<Vec<String>>,
    pub depends_on: Option<Vec<PropertyDependency>>,
    pub roles: Option<Vec<Role>>,
    pub restart_required: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub additional_doc: Option<Vec<String>>,
    pub comment: Option<String>,
    pub description: Option<String>,
}

/// Represents (one of multiple) unique identifier for a property name depending on the type
#[derive(Clone, Debug, Deserialize, Eq, Hash, JsonSchema, PartialOrd, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PropertyName {
    pub name: String,
    pub kind: PropertyNameKind,
}

impl fmt::Display for PropertyName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Represents different config identifier types like config file, environment variable, command line parameter etc.
#[derive(Clone, Debug, Deserialize, Eq, Hash, JsonSchema, PartialOrd, PartialEq)]
#[serde(tag = "type", content = "file", rename_all = "camelCase")]
pub enum PropertyNameKind {
    File(String),
    Env,
    Cli,
}

impl PropertyNameKind {
    pub fn get_file_name(&self) -> String {
        match self {
            PropertyNameKind::File(name) => name.clone(),
            _ => "".to_string(),
        }
    }
}

/// Represents the config unit (name corresponds to the unit type like password and a given regex)
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialOrd, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Unit {
    pub name: String,
    pub regex: String,
    pub examples: Option<Vec<String>>,
    pub comment: Option<String>,
}

/// Represents the default or recommended values a property may have: since default values
/// may change with different releases, optional from and to version parameters can be provided
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialOrd, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PropertyValueSpec {
    pub from_version: Option<String>,
    pub to_version: Option<String>,
    pub value: String,
}

/// Represents all supported data types
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialOrd, PartialEq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub(crate) enum Datatype {
    Bool,
    Integer {
        min: Option<String>,
        max: Option<String>,
        unit: Option<Unit>,
        accepted_units: Option<Vec<String>>,
        default_unit: Option<String>,
    },
    Float {
        min: Option<String>,
        max: Option<String>,
        unit: Option<Unit>,
        accepted_units: Option<Vec<String>>,
        default_unit: Option<String>,
    },
    String {
        min: Option<String>,
        max: Option<String>,
        unit: Option<Unit>,
        accepted_units: Option<Vec<String>>,
        default_unit: Option<String>,
    },
    Array {
        unit: Option<Unit>,
        accepted_units: Option<Vec<String>>,
        default_unit: Option<String>,
    },
}

/// Represents a dependency on another config property and (if available) a required value
/// e.g. to set ssl certificates one has to set some property use_ssl to true
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialOrd, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PropertyDependency {
    pub property_names: Vec<PropertyName>,
    pub value: Option<String>,
}

/// Represents a role in the cluster, e.g. Server / Client and if the property is required
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialOrd, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Role {
    pub name: String,
    pub required: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialOrd, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Foo {
    version: String,
    spec: Spec,
    properties: Vec<PropertyDef>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialOrd, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Spec {
    units: Vec<UnitDef>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialOrd, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UnitDef {
    unit: Unit,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialOrd, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PropertyDef {
    property: PropertySpec,
}

#[cfg(test)]
mod test {
    use super::*;
    use std::error::Error;
    use std::fs;

    #[test]
    fn test_experiment_load_sample_product_config_via_serde() -> Result<(), Box<dyn Error>> {
        let contents = fs::read_to_string("data/test_product_config.yaml")?;
        let product_config: Foo = serde_yaml::from_str(&contents)?;

        println!("{:?}", product_config);
        Ok(())
    }
}
