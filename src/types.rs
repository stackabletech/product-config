use serde::Deserialize;
use std::fmt;

/// represents the root element structure of JSON/YAML documents
#[derive(Deserialize, Debug)]
pub struct ConfigItem {
    pub config_settings: ConfigSetting,
    pub config_options: Vec<ConfigOption>,
}

/// represents config settings like unit and regex specification
#[derive(Deserialize, Debug)]
pub struct ConfigSetting {
    pub unit: Vec<Unit>,
}

/// represents one config entry for a given config property or environmental variable
#[derive(Deserialize, Clone, Debug)]
pub struct ConfigOption {
    pub option_names: Vec<OptionName>,
    pub datatype: Datatype,
    pub default_values: Option<Vec<OptionValue>>,
    pub recommended_values: Option<Vec<OptionValue>>,
    pub allowed_values: Option<Vec<String>>,
    pub as_of_version: String,
    pub deprecated_since: Option<String>,
    pub deprecated_for: Option<Vec<String>>,
    pub depends_on: Option<Vec<Dependency>>,
    pub roles: Option<Vec<Role>>,
    pub restart_required: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub additional_doc: Option<Vec<String>>,
    pub comment: Option<String>,
    pub description: Option<String>,
}

/// represents (one of multiple) unique identifier for a config option depending on the type
#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialOrd, PartialEq)]
pub struct OptionName {
    pub name: String,
    pub kind: OptionKind,
    pub config_file: String,
}

impl fmt::Display for OptionName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// represents different config identifier types like config property, environment variable, command line parameter etc.
#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialOrd, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OptionKind {
    Conf,
    Env,
    Cli,
}

/// represents the config unit (name corresponds to the unit type like password and a given regex)
#[derive(Deserialize, Debug)]
pub struct Unit {
    pub name: String,
    pub regex: Option<String>,
    pub examples: Option<Vec<String>>,
    pub comment: Option<String>,
}

/// represents the default value a config option may have: since default values may change with different releases, optional from and to version parameters can be provided
#[derive(Deserialize, Clone, Debug)]
pub struct OptionValue {
    pub from_version: Option<String>,
    pub to_version: Option<String>,
    pub value: String,
}

/// represents all supported data types
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum Datatype {
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

/// represents a dependency on another config option and (if available) a required value
/// e.g. to set ssl certificates one has to set some property use_ssl to true
#[derive(Deserialize, Clone, Debug)]
pub struct Dependency {
    pub option_names: Vec<OptionName>,
    pub value: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Role {
    pub name: String,
    pub required: bool,
}
