use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;

/// trait for different config readers for json or yaml
pub trait ConfigReader<'a, T>
where
    for<'de> T: Deserialize<'de> + 'a,
{
    fn read(&self) -> Result<T, Box<dyn Error>>;
}

/// specific json config reader struct
pub struct ConfigJsonReader {
    path: String,
}

impl ConfigJsonReader {
    pub fn new(path: String) -> Self {
        ConfigJsonReader { path }
    }
}

impl<'a, T> ConfigReader<'a, T> for ConfigJsonReader
where
    for<'de> T: Deserialize<'de> + 'a,
{
    /// specific json config reader "read" implementation
    fn read(&self) -> Result<T, Box<dyn Error>> {
        let file = File::open(self.path.as_str())?;
        let reader = BufReader::new(file);
        let config_options: T = serde_json::from_reader(reader)?;
        Ok(config_options)
    }
}
