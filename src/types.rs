use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;

/// Represents config spec like unit and regex specification
#[derive(Clone, Debug)]
pub(crate) struct ProductConfigSpecProperties {
    pub units: HashMap<String, Regex>,
}

/// Represents one property spec entry for a given property
#[derive(Deserialize, Clone, Debug)]
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
#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialOrd, PartialEq)]
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
#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialOrd, PartialEq)]
#[serde(tag = "type", content = "file", rename_all = "lowercase")]
pub enum PropertyNameKind {
    File(String),
    Env,
    Cli,
}

impl PropertyNameKind {
    pub fn get_file_name(&self) -> String {
        match self {
            PropertyNameKind::File(conf) => conf.clone(),
            _ => "".to_string(),
        }
    }
}

/// Represents the config unit (name corresponds to the unit type like password and a given regex)
#[derive(Deserialize, Debug)]
pub(crate) struct Unit {
    pub name: String,
    pub regex: Option<String>,
    pub examples: Option<Vec<String>>,
    pub comment: Option<String>,
}

/// Represents the default or recommended values a property may have: since default values
/// may change with different releases, optional from and to version parameters can be provided
#[derive(Deserialize, Clone, Debug, Eq, PartialOrd, PartialEq)]
pub struct PropertyValueSpec {
    pub from_version: Option<String>,
    pub to_version: Option<String>,
    pub value: String,
}

/// Represents all supported data types
#[derive(Deserialize, Clone, Debug, Eq, PartialOrd, PartialEq)]
#[serde(rename_all = "lowercase", tag = "type")]
pub(crate) enum Datatype {
    Bool,
    Integer {
        min: Option<String>,
        max: Option<String>,
        unit: Option<String>,
        accepted_units: Option<Vec<String>>,
        default_unit: Option<String>,
    },
    Float {
        min: Option<String>,
        max: Option<String>,
        unit: Option<String>,
        accepted_units: Option<Vec<String>>,
        default_unit: Option<String>,
    },
    String {
        min: Option<String>,
        max: Option<String>,
        unit: Option<String>,
        accepted_units: Option<Vec<String>>,
        default_unit: Option<String>,
    },
    Array {
        unit: Option<String>,
        accepted_units: Option<Vec<String>>,
        default_unit: Option<String>,
    },
}

/// Represents a dependency on another config property and (if available) a required value
/// e.g. to set ssl certificates one has to set some property use_ssl to true
#[derive(Deserialize, Clone, Debug, Eq, PartialOrd, PartialEq)]
pub struct PropertyDependency {
    pub property_names: Vec<PropertyName>,
    pub value: Option<String>,
}

/// Represents a role in the cluster, e.g. Server / Client and if the property is required
#[derive(Deserialize, Clone, Debug, Eq, PartialOrd, PartialEq)]
pub struct Role {
    pub name: String,
    pub required: bool,
}
