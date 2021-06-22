use crate::error::Error;
//use kube::CustomResource;
//use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductConfig {
    units: Vec<UnitDef>,
    products: Vec<Product>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductConfigStatus {}

/// This is a trade off we have to deal with to allow rust and serde to work with anchor references
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
struct UnitDef {
    unit: Unit,
}

/// Represents the config unit (name corresponds to the unit type like password and a given regex)
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
struct Unit {
    pub name: String,
    pub regex: String,
    pub examples: Option<Vec<String>>,
    pub comment: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
struct Product {
    name: String,
    version: ProductVersion,
    properties: Vec<PropertyDef>,
    commands: Vec<Command>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
struct Command {
    name: String,
    version: ProductVersion,
    command: Vec<String>,
    roles: Vec<String>,
    cli: Vec<Property>,
    files: Vec<File>,
    env: Vec<Property>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
struct File {
    name: String,
    template: FileTemplateName,
    properties: Vec<Property>,
}
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
enum FileTemplateName {
    HadoopXml,
    JavaProperties,
    Value,
}

/// This is a trade off we have to deal with to allow rust and serde to work with anchor references
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
struct PropertyDef {
    // TODO: Not happy with the naming
    property: Property,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
struct Property {
    name: String,
    nullable: bool,
    datatype: Datatype,
    default_values: Option<Vec<PropertyValue>>,
    recommended_values: Option<Vec<PropertyValue>>,
    deprecated: Option<Vec<Deprecated>>,
    depends_on: Option<Vec<PropertyDef>>,
    restart_required: Option<bool>,
    tags: Option<Vec<String>>,
    additional_doc: Option<String>,
    comment: Option<String>,
    description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
struct Deprecated {
    versions: ProductVersion,
    message: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
struct Dependency {
    versions: ProductVersion,
    message: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
struct PropertyValue {
    versions: ProductVersion,
    value: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
struct Datatype {
    kind: DatatypeKind,
    min: Option<String>,
    max: Option<String>,
    unit: Option<Unit>,
    allowed_values: Option<Vec<String>>,
    // TODO: do we need allowed_units?
    //   I think we cover that with different regex patterns in config.units
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
enum DatatypeKind {
    Bool,
    Integer,
    Float,
    String,
    Enum,
    // TODO: I am still missing some kind of collection types like list or map
    //    enum just covers a list with fixed elements
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductVersion {
    from: Option<String>,
    to: Option<String>,
    list: Option<Vec<String>>,
}

/// This is the final product configuration the operator receives.
/// Contains config files, env variables and cli commands / parameters.
#[derive(Clone, Debug)]
// TODO: find better name
pub struct ProductConfiguration {
    // Map<FileName, FileContent>
    pub files: BTreeMap<String, String>,
    // Map<PropertyName, ValidatedValue>
    pub env: BTreeMap<String, String>,
    // e.g. "./start.sh some_command --some_flag --p some_parameter"
    pub cli: String,
}

/// This is required in operator-rs in order to pass the user config and user overrides to
/// the product-config.
pub struct UserConfigAndOverrides {
    config: BTreeMap<String, String>,
    files: BTreeMap<String, BTreeMap<String, String>>,
    env: BTreeMap<String, String>,
    cli: Vec<String>,
}

#[cfg(test)]
mod experiments {

    use crate::experiments::ProductConfig;
    use std::error::Error;
    use std::fs;
    use yaml_rust::YamlLoader;

    #[test]
    fn test_experiment_load_sample_product_config() -> Result<(), Box<dyn Error>> {
        let contents = fs::read_to_string("data/zookeeper.product.config.yaml")?;
        let sample = YamlLoader::load_from_str(contents.as_str())?;
        println!("{:?}", sample);
        assert_eq!(1, sample.len());
        Ok(())
    }

    #[test]
    fn test_experiment_load_sample_product_config_via_serde() -> Result<(), Box<dyn Error>> {
        let contents = fs::read_to_string("data/zookeeper.product.config.yaml")?;
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
