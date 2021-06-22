use crate::experiments::ProductConfig;
use crate::experiments::ProductConfiguration;
use crate::Error;
use std::collections::BTreeMap;
use std::fs;

pub type ProductConfigResult<T> = Result<T, Vec<Error>>;

pub struct ProductConfigManager {
    product_config: ProductConfig,
}

impl ProductConfigManager {
    pub fn from_file(file: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(file)?;
        Ok(ProductConfigManager {
            product_config: serde_yaml::from_str(&contents)?,
        })
    }

    pub fn get(
        &self,
        command: &str,
        role: &str,
        version: &str,
        user_properties: &BTreeMap<String, Option<String>>,
        overrides: Option<BtreeMap<String(Env, CLi, File), BTreeMAp<String, STring>>>,
    ) -> ProductConfigResult<ProductConfiguration> {
        todo!()
    }
}

#[cfg(test)]
mod experiments {
    use crate::experiments::ProductConfig;
    use std::error::Error;
    use std::fs;
    use yaml_rust::YamlLoader;

    #[test]
    fn test_load_sample_product_config() -> Result<(), Box<dyn Error>> {
        let contents = fs::read_to_string("data/zookeeper.product.config.yaml")?;
        let sample = YamlLoader::load_from_str(contents.as_str())?;
        println!("{:?}", sample);
        assert_eq!(1, sample.len());
        Ok(())
    }
}
