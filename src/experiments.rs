#[cfg(test)]
mod experiments {

    use std::error::Error;
    use std::fs;
    use yaml_rust::YamlLoader;

    #[test]
    fn test_experiment_load_sample_product_config() -> Result<(), Box<dyn Error>> {
        let contents = fs::read_to_string("data/sample_product_config.yaml")?;
        let sample = YamlLoader::load_from_str(contents.as_str())?;
        assert_eq!(1, sample.len());
        Ok(())
    }
}
