use serde::Deserialize;
use std::fmt;

/// Represents the root element structure of JSON/YAML documents
#[derive(Deserialize, Debug)]
pub struct ConfigSpec {
    pub config_settings: ConfigSetting,
    pub config_options: Vec<ConfigOption>,
}

/// Represents config settings like unit and regex specification
#[derive(Deserialize, Debug)]
pub struct ConfigSetting {
    pub units: Vec<Unit>,
}

/// Represents one config option entry for a given property
#[derive(Deserialize, Clone, Debug)]
pub struct ConfigOption {
    pub option_names: Vec<ConfigName>,
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

/// Represents (one of multiple) unique identifier for a config option depending on the type
#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialOrd, PartialEq)]
pub struct ConfigName {
    pub name: String,
    pub kind: ConfigKind,
}

impl fmt::Display for ConfigName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Represents different config identifier types like config file, environment variable, command line parameter etc.
#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialOrd, PartialEq)]
#[serde(tag = "type", content = "file", rename_all = "lowercase")]
pub enum ConfigKind {
    Conf(String),
    Env,
    Cli,
}

impl ConfigKind {
    pub fn get_file_name(&self) -> String {
        match self {
            ConfigKind::Conf(conf) => conf.clone(),
            _ => "".to_string(),
        }
    }
}

/// Represents the config unit (name corresponds to the unit type like password and a given regex)
#[derive(Deserialize, Debug)]
pub struct Unit {
    pub name: String,
    pub regex: Option<String>,
    pub examples: Option<Vec<String>>,
    pub comment: Option<String>,
}

/// Represents the default value a config option may have: since default values may change with different releases, optional from and to version parameters can be provided
#[derive(Deserialize, Clone, Debug, Eq, PartialOrd, PartialEq)]
pub struct OptionValue {
    pub from_version: Option<String>,
    pub to_version: Option<String>,
    pub value: String,
}

/// Represents all supported data types
#[derive(Deserialize, Clone, Debug, Eq, PartialOrd, PartialEq)]
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

/// Represents a dependency on another config option and (if available) a required value
/// e.g. to set ssl certificates one has to set some property use_ssl to true
#[derive(Deserialize, Clone, Debug, Eq, PartialOrd, PartialEq)]
pub struct Dependency {
    pub option_names: Vec<ConfigName>,
    pub value: Option<String>,
}

/// Represents a role in the cluster, e.g. Server / Client and if the
/// config option is required
#[derive(Deserialize, Clone, Debug, Eq, PartialOrd, PartialEq)]
pub struct Role {
    pub name: String,
    pub required: bool,
}
