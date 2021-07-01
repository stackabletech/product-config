use crate::types::{PropertyNameKind, PropertySpec};
use crate::validation::ValidationResult;
use semver::Version;
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

/// Helper to check if any given key is contained in a map.
pub(crate) fn hashmap_contains_any_key<K, V>(hm: &HashMap<K, V>, possible_keys: &[K]) -> bool
where
    K: Hash + Eq,
{
    for key in possible_keys {
        if hm.contains_key(key) {
            return true;
        }
    }
    false
}

/// Checks if the provided property has other properties which need to be expanded / added in
/// order to work correctly. If any expanded properties are available, they are checked for
/// a fitting role and version and added to the result if role and version are matching.
///
/// # Arguments
/// * `property` - the property that may have other properties to expand to
/// * `version` - the current product version
/// * `role` - property role provided by the user
/// * `kind` - property name kind provided by the user
pub(crate) fn expand_properties(
    property: &PropertySpec,
    version: &Version,
    role: &str,
    kind: &PropertyNameKind,
) -> ValidationResult<BTreeMap<String, Option<String>>> {
    let mut result = BTreeMap::new();
    if let Some(expands_to) = &property.expands_to {
        for to_expand in expands_to {
            if !to_expand.property.has_role(role) {
                continue;
            }

            if !to_expand.property.is_version_supported(version)? {
                continue;
            }

            if let Some(name) = to_expand.property.name_from_kind(kind) {
                if to_expand.value.is_some() {
                    result.insert(name, to_expand.value.clone());
                } else if let Some((_, value)) =
                    to_expand.property.recommended_or_default(version, kind)
                {
                    result.insert(name, value);
                }
            }
        }
    }
    Ok(result)
}
