//! A config reader implementation to abstract the source of config data
//!
//! For now only JSON as source is supported.
//!
//! Possible extensions: YAML, CSV, database ...
use crate::error::Error;
use serde::de::DeserializeOwned;
use std::fs::File;
use std::io::BufReader;

/// trait for different config readers for json or yaml
pub trait ConfigReader<T: DeserializeOwned> {
    fn read(&self) -> Result<T, Error>;
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

impl<T: DeserializeOwned> ConfigReader<T> for ConfigJsonReader {
    /// specific json config reader "read" implementation
    fn read(&self) -> Result<T, Error> {
        let file = match File::open(self.path.as_str()) {
            Ok(file) => file,
            Err(_) => {
                return Err(Error::FileNotFound {
                    file_name: self.path.to_string(),
                });
            }
        };

        let reader = BufReader::new(file);
        match serde_json::from_reader(reader) {
            Ok(t) => Ok(t),
            Err(err) => Err(Error::FileNotParsable {
                file_name: self.path.to_string(),
                reason: err.to_string(),
            }),
        }
    }
}
