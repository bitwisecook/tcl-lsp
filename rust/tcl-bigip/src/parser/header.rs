//! Stanza-header parsing — `(module, object_type, identifier)` resolution.
//!
//! Multi-word object types (`sys diags ihealth`, `ltm message-routing
//! diameter route`) are resolved by longest-prefix match against the
//! BIG-IP registry's known object types per module; the remaining tail is
//! the identifier. An empty identifier denotes a global singleton.

use std::collections::{HashMap, HashSet};

use tcl_registry::bigip::BigipRegistry;

use super::helpers::tokenise_header;

/// Per-module set of registered object-type strings, for longest-prefix
/// header resolution.
#[derive(Debug, Clone)]
pub struct ObjectTypeIndex {
    by_module: HashMap<&'static str, HashSet<&'static str>>,
}

impl ObjectTypeIndex {
    /// Build the index from the BIG-IP registry's header types.
    #[must_use]
    pub fn build(registry: &BigipRegistry) -> Self {
        let mut by_module: HashMap<&'static str, HashSet<&'static str>> = HashMap::new();
        for spec in registry.specs() {
            for &(module, object_type) in spec.header_types {
                by_module.entry(module).or_default().insert(object_type);
            }
        }
        Self { by_module }
    }

    fn known(&self, module: &str) -> Option<&HashSet<&'static str>> {
        self.by_module.get(module)
    }
}

/// Parse a stanza header into `(module, object_type, identifier)`.
/// Returns `None` for headers with fewer than two tokens.
#[must_use]
pub fn parse_generic_header(
    header: &str,
    index: &ObjectTypeIndex,
) -> Option<(String, String, String)> {
    let parts = tokenise_header(header);
    if parts.len() < 2 {
        return None;
    }
    let module = parts[0].clone();
    if parts.len() == 2 {
        return Some((module, parts[1].clone(), String::new()));
    }

    if let Some(known) = index.known(&module) {
        for prefix_len in (2..parts.len()).rev() {
            let candidate = parts[1..prefix_len].join(" ");
            if known.contains(candidate.as_str()) {
                let identifier = parts[prefix_len..].join(" ");
                return Some((module, candidate, identifier));
            }
        }
        let whole = parts[1..].join(" ");
        if known.contains(whole.as_str()) {
            return Some((module, whole, String::new()));
        }
    }

    // Heuristic fallback: last token is the identifier.
    let object_type = parts[1..parts.len() - 1].join(" ");
    if object_type.is_empty() {
        let id = parts.get(2).cloned().unwrap_or_default();
        return Some((module, parts[1].clone(), id));
    }
    let identifier = parts[parts.len() - 1].clone();
    Some((module, object_type, identifier))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_named_and_singleton_headers() {
        let registry = BigipRegistry::build();
        let index = ObjectTypeIndex::build(&registry);
        assert_eq!(
            parse_generic_header("ltm pool /Common/p1", &index),
            Some(("ltm".into(), "pool".into(), "/Common/p1".into()))
        );
        // Multi-word singleton object type, no identifier.
        assert_eq!(
            parse_generic_header("sys diags ihealth", &index),
            Some(("sys".into(), "diags ihealth".into(), String::new()))
        );
    }
}
