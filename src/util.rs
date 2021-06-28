use crate::error::Error;
use crate::types::{
    PropertyDependency, PropertyName, PropertyNameKind, PropertySpec, PropertyValueSpec,
};
use crate::validation::ValidationResult;
use semver::Version;
use std::collections::HashMap;

/// This is a helper method to merge SemVer errors and the product config errors. Since
/// SemVer 1.0.X we can no longer use "thiserror" in combination with "#[from]" on the SemVer
/// error (Clone, PartialOrd, PartialEq traits are no longer valid). Therefore we just pass
/// the error string of the SemVer error into our product config error.
pub(crate) fn semver_parse(version: &str) -> ValidationResult<Version> {
    match Version::parse(version) {
        Ok(version) => Ok(version),
        Err(err) => Err(Error::InvalidVersion {
            semver_error: err.to_string(),
        }),
    }
}

pub(crate) fn hashmap_contains_any_key<K, V>(hm: &HashMap<K, V>, possible_keys: Vec<K>) -> bool {
    for key in possible_keys {
        if hm.contains_key(key) {
            return true;
        }
    }
    false
}
