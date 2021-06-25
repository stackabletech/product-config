use std::cmp::Ordering;
use std::collections::HashMap;
use std::iter::FromIterator;
use std::ops::Deref;
use std::{fmt, ops};

use regex::Regex;
use schemars::gen::SchemaGenerator;
use schemars::schema::Schema;
use schemars::JsonSchema;
use semver::Version;
use serde::{de, Deserialize, Deserializer};

use crate::util::semver_parse;
use crate::validation::ValidationResult;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialOrd, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProductConfig {
    pub version: String,
    pub spec: Spec,
    pub properties: Vec<PropertyAnchor>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialOrd, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Spec {
    units: Vec<UnitAnchor>,
}

/// This is a workaround to use yaml anchors with serde
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialOrd, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UnitAnchor {
    pub unit: Unit,
}

/// This is a workaround to use yaml anchors with serde
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialOrd, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PropertyAnchor {
    pub property: PropertySpec,
}

impl ops::Deref for PropertyAnchor {
    type Target = PropertySpec;
    fn deref(&self) -> &PropertySpec {
        &self.property
    }
}

/// Represents one property spec entry for a given property
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialOrd, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PropertySpec {
    pub property_names: Vec<PropertyName>,
    pub datatype: Datatype,
    pub as_of_version: String,
    pub default_values: Option<Vec<PropertyValueSpec>>,
    pub recommended_values: Option<Vec<PropertyValueSpec>>,
    pub allowed_values: Option<Vec<String>>,
    pub deprecated_since: Option<String>,
    pub deprecated_for: Option<Vec<String>>,
    pub expands_to: Option<Vec<PropertyDependency>>,
    pub roles: Option<Vec<Role>>,
    pub restart_required: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub additional_doc: Option<Vec<String>>,
    pub comment: Option<String>,
    pub description: Option<String>,
}

impl PropertySpec {
    pub fn recommended_or_default(
        &self,
        version: &Version,
        kind: &PropertyNameKind,
    ) -> Option<(String, Option<String>)> {
        if let Some(name) = self.name_from_kind(kind) {
            if let Some(recommended_vals) = &self.recommended_values {
                let val = self.filter_value(version, recommended_vals);
                return Some((name, val));
            } else if let Some(default_vals) = &self.default_values {
                let val = self.filter_value(version, default_vals);
                return Some((name, val));
            }
        }
        None
    }

    fn filter_value(&self, version: &Version, values: &[PropertyValueSpec]) -> Option<String> {
        for value in values {
            if let Some(from) = &value.from_version {
                let from_version = semver_parse(from).unwrap();

                if from_version > *version {
                    continue;
                }
            }

            if let Some(to) = &value.to_version {
                let to_version = semver_parse(to).unwrap();

                if to_version < *version {
                    continue;
                }
            }

            return Some(value.value.clone());
        }
        None
    }

    pub fn name_from_kind(&self, kind: &PropertyNameKind) -> Option<String> {
        for name in &self.property_names {
            if name.kind == *kind {
                return Some(name.name.to_string());
            }
        }
        None
    }

    pub fn has_role_required(&self, user_role: &str) -> bool {
        if let Some(roles) = &self.roles {
            for role in roles {
                if role.name == user_role && role.required {
                    return true;
                }
            }
        }
        false
    }

    pub fn has_role(&self, user_role: &str) -> bool {
        if let Some(roles) = &self.roles {
            for role in roles {
                if role.name == user_role {
                    return true;
                }
            }
        }
        false
    }

    pub fn is_version_supported(&self, product_version: &Version) -> ValidationResult<bool> {
        Ok(semver_parse(&self.as_of_version)? <= *product_version)
    }

    pub fn is_version_deprecated(&self, product_version: &Version) -> ValidationResult<bool> {
        if let Some(deprecated_since) = &self.deprecated_since {
            return Ok(semver_parse(deprecated_since)? <= *product_version);
        }
        Ok(false)
    }

    pub fn property_name(&self, kind: &PropertyNameKind) -> Vec<String> {
        Vec::from_iter(
            self.property_names
                .iter()
                .map(|property| property.name.clone()),
        )
    }
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
pub struct Unit {
    pub name: String,
    #[serde(deserialize_with = "regex_from_string")]
    pub regex: StackableRegex,
    pub examples: Option<Vec<String>>,
    pub comment: Option<String>,
}

fn regex_from_string<'de, D>(deserializer: D) -> Result<StackableRegex, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let r = Regex::new(&s).map_err(de::Error::custom)?;
    Ok(StackableRegex {
        expression: s,
        compiled: r,
    })
}

/// This is a workaround to deserialize a String directly into a compiled regex.
/// Regex does not implement Eq, PartialOrd, PartialEq and JsonSchema.
/// The field "compiled" should be hidden and only kept in memory. Never to be Serialized
/// or explicitly Deserialized.
// TODO: When moving to custom resources we need to properly implement JsonSchema
//    e.g. map expression back to "regex" field string
#[derive(Clone, Debug)]
pub struct StackableRegex {
    pub expression: String,
    compiled: Regex,
}

impl ops::Deref for StackableRegex {
    type Target = regex::Regex;
    fn deref(&self) -> &regex::Regex {
        &self.compiled
    }
}

impl Eq for StackableRegex {}
impl PartialOrd for StackableRegex {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.expression.partial_cmp(&other.expression)
    }
}

impl PartialEq for StackableRegex {
    fn eq(&self, other: &Self) -> bool {
        self.expression == other.expression
    }
}

impl JsonSchema for StackableRegex {
    fn schema_name() -> String {
        todo!()
    }
    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        todo!()
    }
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
pub enum Datatype {
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
    pub property: PropertySpec,
    pub value: Option<String>,
}

/// Represents a role in the cluster, e.g. Server / Client and if the property is required
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialOrd, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Role {
    pub name: String,
    pub required: bool,
}

#[cfg(test)]
mod test {
    use std::error::Error;
    use std::fs;

    use super::*;

    #[test]
    fn test_experiment_load_sample_product_config_via_serde() -> Result<(), Box<dyn Error>> {
        let contents = fs::read_to_string("data/test_product_config.yaml")?;
        let product_config: ProductConfig = serde_yaml::from_str(&contents)?;

        println!("{:?}", product_config);
        Ok(())
    }
}
