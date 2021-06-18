use crate::error::Error;
//use kube::CustomResource;
//use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    products: Vec<Product>,
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
struct Product {
    name: String,
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
    // TODO: Thought process here: If we want to write to a file we need sth to write, so no
    //    option in my opinion
    properties: Vec<ApplicationProperty>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
struct Env {
    properties: Vec<ApplicationProperty>,
}

/// This is a trade off we have to deal with to allow rust and serde to work with anchor references
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
struct ApplicationProperty {
    // TODO: Not happy with the naming
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
    //  TODO: I like tags for searching / sorting. I think the mix between doc, comment, description
    //   might be some overkill. Additional_docs was for url or links related to the property,
    //   comment just something random or helpful and description is the actual property description.
    //   Come to think of it maybe we just keep it like that.
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
    // TODO: do we need allowed_units?
    //   I think we cover that with different regex patterns in config.units
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[serde(rename_all = "lowercase")]
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
struct ProductVersion {
    from: Option<String>,
    to: Option<String>,
    list: Option<Vec<String>>,
}

/// This is the final product configuration the operator receives.
/// Contains config files, env variables and cli commands / parameters.
/// It is up to the operator to decide what to do with the PropertyValidationResult.
#[derive(Clone, Debug)]
pub struct ProductConfiguration {
    // Map<FileName, Map<Property, ValidatedValue>
    // TODO: How do we introduce the templates here?
    //   If we use templates we can not work on that type here.
    pub files: Option<BTreeMap<String, BTreeMap<String, PropertyValidationResult>>>,
    // Map<Property, ValidatedValue>
    pub env: Option<BTreeMap<String, Option<PropertyValidationResult>>>,
    // e.g. ["./start.sh", "some_command", "--some_flag", "-p", "some_parameter"]
    pub cli: Vec<String>,
}

/// This will be returned for every validated configuration value (including user values
/// and automatically added values from e.g. dependency, recommended etc.).
#[derive(Clone, Debug, PartialOrd, PartialEq)]
pub enum PropertyValidationResult {
    /// On Default, the provided value does not differ from the default settings and may be
    /// left out from the user config in the future.
    Default(String),
    /// On RecommendedDefault, the value for this configuration property is a recommended value.
    /// Will be returned when the user did not provide a value and the product does not have a default.
    RecommendedDefault(String),
    /// On Valid, the value passed all checks and can be used.
    Valid(String),
    /// On warn, the value maybe used with caution.
    Warn(String, Error),
    /// On error, check the provided config and config values.
    /// Should never be used like this!
    Error(Error),
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
