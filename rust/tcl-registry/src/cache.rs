//! Per-dialect [`CommandRegistry`] cache.
//!
//! Each canonical dialect gets one lazily-built, cached `&'static`
//! registry so the per-call build cost is paid once. Unparseable
//! dialect strings collapse to the plain-Tcl entry so a stream of typos
//! cannot leak one registry per typo.
//!
//! Lives in `tcl-registry` (not a consumer crate) so every downstream
//! tool — the CLI, the compiler explorer, future MCP/AI surfaces — shares
//! one cache rather than each rebuilding its own.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::dialects::DialectSet;
use crate::registry::CommandRegistry;

/// Return the cached registry for `dialect`, building it on first use.
pub fn registry_for_dialect(dialect: &str) -> &'static CommandRegistry {
    static REGISTRIES: OnceLock<Mutex<HashMap<String, &'static CommandRegistry>>> = OnceLock::new();

    let parsed = DialectSet::parse(dialect);
    // Canonicalise the cache key: parseable dialects keep their string;
    // unparseable ones share the plain-Tcl entry.
    let key = if parsed.is_some() { dialect } else { "" };

    let map = REGISTRIES.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = map.lock().expect("registry cache mutex");
    if let Some(r) = guard.get(key) {
        return r;
    }

    let mut registry = CommandRegistry::build_default();
    if let Some(d) = parsed {
        registry.load_dialect(d);
    }
    let leaked: &'static CommandRegistry = Box::leak(Box::new(registry));
    guard.insert(key.to_owned(), leaked);
    leaked
}
