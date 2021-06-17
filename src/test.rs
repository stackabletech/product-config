use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, CustomResource, Debug, Deserialize, JsonSchema, Serialize)]
#[kube(
    group = "productconfig.stackable.tech",
    version = "v1",
    kind = "ProductConfig",
    shortname = "pc",
    namespaced
)]
#[kube(status = "ProductConfigStatus")]
#[serde(rename_all = "camelCase")]
pub(crate) struct Spec {
    properties: Vec<SpecTest>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
pub(crate) struct ProductConfigStatus {}

/// Represents one property spec entry for a given property
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
pub(crate) struct SpecTest {
    pub property_name: String,
    //#[serde::pattern = ".*"]
    pub file: Option<Vec<String>>,
    pub env: Option<Vec<String>>,
    pub cli_arg: Option<Vec<String>>,
    pub default_values: Option<Vec<PropertyValueSpec1>>,
    pub recommended_values: Option<Vec<PropertyValueSpec1>>,
}

/// Represents the default or recommended values a property may have: since default values
/// may change with different releases, optional from and to version parameters can be provided
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialOrd, PartialEq, Serialize)]
pub struct PropertyValueSpec1 {
    pub from_version: Option<String>,
    pub to_version: Option<String>,
    pub value: String,
}

/// Represents different config identifier types like config file, environment variable, command line parameter etc.
#[derive(Clone, Debug, Deserialize, Hash, Eq, JsonSchema, PartialOrd, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EnumTest {
    File,
    Env,
    Cli,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml;
    use std::fs;

    #[test]
    fn generate_crds() {
        let target_file = "data/test.yaml";
        let schema = ProductConfig::crd();
        let string_schema = match serde_yaml::to_string(&schema) {
            Ok(schema) => schema,
            Err(err) => panic!("Failed to retrieve CRD: [{}]", err),
        };
        match fs::write(target_file, string_schema) {
            Ok(()) => println!("Successfully wrote CRD to file."),
            Err(err) => println!("Failed to write file: [{}]", err),
        }
    }
}
