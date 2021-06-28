use crate::error::Error;
use crate::types::{PropertyNameKind, PropertySpec};
use crate::validation::ValidationResult;
use semver::Version;
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

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

pub(crate) fn hashmap_contains_any_key<K, V>(hm: &HashMap<K, V>, possible_keys: Vec<K>) -> bool
where
    K: Hash + Eq,
{
    for key in &possible_keys {
        if hm.contains_key(key) {
            return true;
        }
    }
    false
}

pub(crate) fn expand_properties(
    property: &PropertySpec,
    version: &Version,
    role: &str,
    kind: &PropertyNameKind,
) -> ValidationResult<BTreeMap<String, Option<String>>> {
    let mut result = BTreeMap::new();
    if let Some(expands_to) = &property.expands_to {
        for dependency in expands_to {
            if !dependency.property.has_role(role) {
                continue;
            }

            if !dependency.property.is_version_supported(version)? {
                continue;
            }

            if let Some(name) = dependency.property.name_from_kind(kind) {
                if dependency.value.is_some() {
                    result.insert(name, dependency.value.clone());
                } else if let Some((_, value)) =
                    dependency.property.recommended_or_default(version, kind)
                {
                    result.insert(name, value);
                }
            }
        }
    }
    Ok(result)
}
