use std::cmp::Ordering;
use std::{fmt, ops};

use fancy_regex::Regex;
use schemars::gen::SchemaGenerator;
use schemars::schema::Schema;
use schemars::JsonSchema;
use semver::Version;
use serde::{de, Deserialize, Deserializer, Serializer};

use crate::error;
use crate::validation::ValidationResult;
use std::ops::Deref;

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
    pub roles: Vec<Role>,
    #[serde(deserialize_with = "version_from_string")]
    #[serde(serialize_with = "version_to_string")]
    pub as_of_version: StackableVersion,
    pub default_values: Option<Vec<PropertyValueSpec>>,
    pub recommended_values: Option<Vec<PropertyValueSpec>>,
    pub allowed_values: Option<Vec<String>>,
    #[serde(default)]
    #[serde(deserialize_with = "optional_version_from_string")]
    #[serde(serialize_with = "optional_version_to_string")]
    pub deprecated_since: Option<StackableVersion>,
    pub deprecated_for: Option<Vec<String>>,
    pub expands_to: Option<Vec<PropertyExpansion>>,
    pub restart_required: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub additional_doc: Option<Vec<String>>,
    pub comment: Option<String>,
    pub description: Option<String>,
}

impl PropertySpec {
    /// Extract the (preferred) recommended or default value from the property that matches
    /// the provided version.
    pub fn recommended_or_default(
        &self,
        version: &Version,
        kind: &PropertyNameKind,
    ) -> Option<(String, Option<String>)> {
        if let Some(name) = self.name_from_kind(kind) {
            return if let Some(recommended_vals) = &self.recommended_values {
                let val = self.filter_value(version, recommended_vals);
                Some((name, val))
            } else if let Some(default_vals) = &self.default_values {
                let val = self.filter_value(version, default_vals);
                Some((name, val))
            } else {
                Some((name, None))
            };
        }
        None
    }

    /// Filters a recommended or default [`PropertyValueSpec`] to match the provided version
    /// via its to and from range.
    pub fn filter_value(&self, version: &Version, values: &[PropertyValueSpec]) -> Option<String> {
        for value in values {
            if let Some(from) = &value.from_version {
                let from_version = from.deref();

                if from_version > version {
                    continue;
                }
            }

            if let Some(to) = &value.to_version {
                let to_version = to.deref();

                if to_version < version {
                    continue;
                }
            }

            return Some(value.value.clone());
        }
        None
    }

    /// Returns the property name by matching the provided kind. There should be only one reference
    /// to CLI and ENV, as well as multiple references to FILE(s) with different names.
    pub fn name_from_kind(&self, kind: &PropertyNameKind) -> Option<String> {
        for name in &self.property_names {
            if name.kind == *kind {
                return Some(name.name.to_string());
            }
        }
        None
    }

    /// Returns true if the role matches and no_copy is set to true.
    pub fn has_role_no_copy(&self, user_role: &str) -> bool {
        for role in &self.roles {
            if role.name == user_role && role.no_copy == Some(true) {
                return true;
            }
        }
        false
    }

    /// Returns true if the role matches is required.
    pub fn has_role_required(&self, user_role: &str) -> bool {
        for role in &self.roles {
            if role.name == user_role && role.required {
                return true;
            }
        }
        false
    }

    /// Returns true if the role matches.
    pub fn has_role(&self, user_role: &str) -> bool {
        for role in &self.roles {
            if role.name == user_role {
                return true;
            }
        }
        false
    }

    /// Returns true if the product_version is greater or equal the as_of_version of the property.
    pub fn is_version_supported(&self, product_version: &Version) -> ValidationResult<bool> {
        Ok(self.as_of_version.deref() <= product_version)
    }

    /// Returns true if the product_version is greater or equal the deprecated_since of the property.
    pub fn is_version_deprecated(&self, product_version: &Version) -> ValidationResult<bool> {
        if let Some(deprecated_since) = &self.deprecated_since {
            return Ok(deprecated_since.deref() <= product_version);
        }
        Ok(false)
    }

    /// Returns all known property names.
    pub fn all_property_names(&self) -> Vec<String> {
        self.property_names
            .iter()
            .map(|pn| pn.name.clone())
            .collect()
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

/// This is a workaround to deserialize a string directly into a parsed SemVer version and to
/// wrap SemVer in case of using another library.
#[derive(Clone, Debug, Eq, PartialOrd, PartialEq)]
pub struct StackableVersion {
    version: Version,
}

impl StackableVersion {
    pub fn parse(version: &str) -> ValidationResult<Self> {
        Ok(StackableVersion {
            version: Version::parse(version).map_err(|err| error::Error::InvalidVersion {
                version: version.to_string(),
                reason: err.to_string(),
            })?,
        })
    }
}

impl ops::Deref for StackableVersion {
    type Target = Version;
    fn deref(&self) -> &Version {
        &self.version
    }
}

pub fn version_to_string<S>(version: &StackableVersion, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&version.deref().to_string())
}

pub fn optional_version_to_string<S>(
    version: &Option<StackableVersion>,
    s: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if let Some(ref v) = *version {
        return s.serialize_str(&v.deref().to_string());
    }
    s.serialize_none()
}

fn version_from_string<'de, D>(deserializer: D) -> Result<StackableVersion, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    StackableVersion::parse(&s).map_err(de::Error::custom)
}

fn optional_version_from_string<'de, D>(
    deserializer: D,
) -> Result<Option<StackableVersion>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    if let Some(val) = s {
        return Ok(Some(
            StackableVersion::parse(&val).map_err(de::Error::custom)?,
        ));
    }
    Ok(None)
}

impl JsonSchema for StackableVersion {
    fn schema_name() -> String {
        todo!()
    }
    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        todo!()
    }
}

/// This is a workaround to deserialize a string directly into a compiled regex.
/// It is needed because Regex does not implement Eq, PartialOrd, PartialEq and JsonSchema.
/// The field "compiled" should be hidden and only kept in memory. Never to be Serialized
/// or explicitly Deserialized.
// TODO: When moving to custom resources we need to properly implement JsonSchema
//    e.g. map expression back to "regex" field string
#[derive(Clone, Debug)]
pub struct StackableRegex {
    pub expression: String,
    compiled: Regex,
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

impl ops::Deref for StackableRegex {
    type Target = Regex;
    fn deref(&self) -> &Regex {
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
    #[serde(default)]
    #[serde(deserialize_with = "optional_version_from_string")]
    #[serde(serialize_with = "optional_version_to_string")]
    pub from_version: Option<StackableVersion>,
    #[serde(default)]
    #[serde(deserialize_with = "optional_version_from_string")]
    #[serde(serialize_with = "optional_version_to_string")]
    pub to_version: Option<StackableVersion>,
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

/// Represents an expansion on another config property and (if available) a required value
/// e.g. to set ssl certificates one has to set some property use_ssl to true
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialOrd, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PropertyExpansion {
    pub property: PropertySpec,
    pub value: Option<String>,
}

/// Represents a role in the cluster, e.g. Server / Client and if the property is required
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialOrd, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Role {
    pub name: String,
    pub required: bool,
    pub no_copy: Option<bool>,
}
