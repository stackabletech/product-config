use kube::CustomResource;
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// #[derive(Clone, CustomResource, Debug, Deserialize, JsonSchema, Serialize)]
// #[kube(
//     group = "productconfig.stackable.tech",
//     version = "v1",
//     kind = "ProductConfig",
//     shortname = "pc",
//     namespaced
// )]
// #[kube(status = "ProductConfigStatus")]
// #[serde(rename_all = "camelCase")]
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
//pub struct ProductConfigSpec {
pub struct ProductConfig {
    config: Config,
    applications: Vec<Application>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
pub struct ProductConfigStatus {}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
struct Config {
    units: Vec<ConfigUnit>,
}

/// This is a trade off we have to deal with to allow rust and serde to work with anchor references
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
struct ConfigUnit {
    unit: Unit,
}

/// Represents the config unit (name corresponds to the unit type like password and a given regex)
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
struct Unit {
    pub name: String,
    pub regex: String,
    pub examples: Option<Vec<String>>,
    pub comment: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
struct Application {
    product: String,
    roles: Vec<String>,
    versions: ProductVersion,
    cli: Cli,
    file: Option<File>,
    env: Option<Env>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
struct Cli {
    command: String,
    properties: Option<Vec<ApplicationProperty>>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
struct File {
    name: String,
    template: Option<String>,
    properties: Vec<ApplicationProperty>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
struct Env {
    properties: Vec<ApplicationProperty>,
}

/// This is a trade off we have to deal with to allow rust and serde to work with anchor references
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
struct ApplicationProperty {
    property: Property,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
struct Property {
    name: String,
    nullable: bool,
    datatype: Datatype,
    default_values: Option<Vec<PropertyValue>>,
    recommended_values: Option<Vec<PropertyValue>>,
    deprecated: Option<Vec<Deprecated>>,
    depends_on: Option<Vec<ApplicationProperty>>,
    restart_required: Option<bool>,
    tags: Option<Vec<String>>,
    additional_doc: Option<String>,
    comment: Option<String>,
    description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
struct Deprecated {
    versions: ProductVersion,
    message: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
struct Dependency {
    versions: ProductVersion,
    message: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
struct PropertyValue {
    versions: ProductVersion,
    value: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
struct Datatype {
    kind: DatatypeKind,
    min: Option<String>,
    max: Option<String>,
    unit: Option<Unit>,
    allowed_values: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "lowercase")]
enum DatatypeKind {
    Bool,
    Integer,
    Float,
    String,
    Enum,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
struct ProductVersion {
    from: Option<String>,
    to: Option<String>,
    list: Option<Vec<String>>,
}

#[cfg(test)]
mod experiments {

    use crate::experiments::ProductConfig;
    use std::error::Error;
    use std::fs;
    use yaml_rust::YamlLoader;

    #[test]
    fn test_experiment_load_sample_product_config() -> Result<(), Box<dyn Error>> {
        let contents = fs::read_to_string("data/sample_product_config.yaml")?;
        let sample = YamlLoader::load_from_str(contents.as_str())?;
        println!("{:?}", sample);
        assert_eq!(1, sample.len());
        Ok(())
    }

    #[test]
    fn test_experiment_load_sample_product_config_via_serde() -> Result<(), Box<dyn Error>> {
        let contents = fs::read_to_string("data/sample_product_config.yaml")?;
        let product_config: ProductConfig = serde_yaml::from_str(&contents)?;

        println!("{:?}", product_config);
        Ok(())
    }

    #[test]
    fn generate_crds() {
        // let target_file = "data/test.yaml";
        // let schema = ProductConfig::crd();
        // let string_schema = match serde_yaml::to_string(&schema) {
        //     Ok(schema) => schema,
        //     Err(err) => panic!("Failed to retrieve CRD: [{}]", err),
        // };
        // match fs::write(target_file, string_schema) {
        //     Ok(()) => println!("Successfully wrote CRD to file."),
        //     Err(err) => println!("Failed to write file: [{}]", err),
        // }
    }

    #[test]
    fn generate_cr() -> Result<(), Box<dyn Error>> {
        // let contents = fs::read_to_string("data/sample_product_config.yaml")?;
        // let product_config: ProductConfig = serde_yaml::from_str(&contents)?;
        //
        // let target_file = "data/test_cr.yaml";
        // let string_schema = match serde_yaml::to_string(&product_config) {
        //     Ok(schema) => schema,
        //     Err(err) => panic!("Failed to retrieve CR: [{}]", err),
        // };
        // match fs::write(target_file, string_schema) {
        //     Ok(()) => println!("Successfully wrote CR to file."),
        //     Err(err) => println!("Failed to write file: [{}]", err),
        // }
        Ok(())
    }
}
