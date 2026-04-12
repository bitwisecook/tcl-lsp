//! Memory-SSA: versioned memory operations with alias analysis.
//!
//! Memory-SSA extends scalar SSA to track *memory locations* — stores
//! and loads that go through aliased bindings (`upvar`, `global`,
//! `variable`, `namespace upvar`).
//!
//! Key concepts:
//!
//! - [`MemoryLocation`] identifies *what* is being stored/loaded
//!   (local, upvar alias, global, namespace variable, array element).
//! - [`AliasSet`] groups [`MemoryLocation`]s that may refer to the
//!   same underlying storage.
//! - [`MemoryOp`] annotates statements with versioned memory
//!   operations so that downstream passes (GVN, DSE, copy
//!   propagation) can reason about aliased state precisely.
//!
//! This module operates *after* scalar SSA construction (C3/C6) and
//! inspects the IR for aliasing commands (`upvar`, `global`,
//! `variable`, `namespace upvar`) + barriers (`eval`/`uplevel`).
//!
//! Ported from `core/compiler/memory_ssa.py` in three strips:
//! - **C24b1** (this file) — location types and alias-set queries.
//! - **C24b2** — memory-op types + `MemorySSAFunction` + detection
//!   helpers.
//! - **C24b3** — `compute_aliases` + `build_memory_ssa` driver.

use std::collections::BTreeSet;

// ---------------------------------------------------------------------------
// MemoryLocationKind / MemoryLocation (C24b1)
// ---------------------------------------------------------------------------

/// Classification of a memory location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MemoryLocationKind {
    /// Procedure-local variable (no aliasing concerns).
    Local,
    /// Aliased from a caller frame via `upvar`.
    Upvar,
    /// Global variable (`global` command or `::` prefix).
    Global,
    /// Namespace-scoped variable (`variable` or `namespace upvar`).
    NamespaceVar,
    /// Element of a Tcl array (`arrayName(index)`).
    ArrayElement,
    /// OO instance variable (`variable` in method body).
    InstanceVar,
    /// Cannot be determined statically.
    Unknown,
}

/// A specific memory location in the program.
///
/// The `qualifier` field carries location-specific context:
/// namespace (for [`MemoryLocationKind::NamespaceVar`]), caller-side
/// variable name (for [`MemoryLocationKind::Upvar`]), or array index
/// text (for [`MemoryLocationKind::ArrayElement`]).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MemoryLocation {
    /// Classification of the location.
    pub kind: MemoryLocationKind,
    /// Variable name.
    pub name: String,
    /// Location-specific context.
    pub qualifier: String,
}

impl MemoryLocation {
    /// Build a location with `kind`, `name`, and an empty qualifier.
    #[must_use]
    pub fn new(kind: MemoryLocationKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
            qualifier: String::new(),
        }
    }

    /// Build a location with an explicit qualifier.
    #[must_use]
    pub fn with_qualifier(
        kind: MemoryLocationKind,
        name: impl Into<String>,
        qualifier: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            name: name.into(),
            qualifier: qualifier.into(),
        }
    }

    /// Render the location in the same format used by the Python
    /// `MemoryLocation.__str__` for diagnostic parity.
    #[must_use]
    pub fn display(&self) -> String {
        match self.kind {
            MemoryLocationKind::Upvar => {
                format!("upvar({} -> {})", self.qualifier, self.name)
            }
            MemoryLocationKind::Global => format!("global({})", self.name),
            MemoryLocationKind::NamespaceVar => {
                format!("ns({}::{})", self.qualifier, self.name)
            }
            MemoryLocationKind::InstanceVar => {
                format!("ivar({}::{})", self.qualifier, self.name)
            }
            MemoryLocationKind::ArrayElement => {
                format!("{}({})", self.name, self.qualifier)
            }
            MemoryLocationKind::Unknown => format!("?({})", self.name),
            MemoryLocationKind::Local => self.name.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// AliasSet (C24b1)
// ---------------------------------------------------------------------------

/// A group of memory locations that may alias each other.
///
/// For example, if `upvar 1 caller_x local_x` is in scope then
/// `caller_x` and `local_x` form an alias set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasSet {
    /// Locations merged into this set. Ordered for stable output.
    pub locations: BTreeSet<MemoryLocation>,
    /// Reason describing why the set was formed — e.g. `"upvar"`,
    /// `"global"`, `"variable"`, or a combination (comma-separated,
    /// sorted) when multiple detection paths merged into the same
    /// set.
    pub reason: String,
}

impl AliasSet {
    /// Construct an alias set from an owned location set + reason.
    #[must_use]
    pub fn new(locations: BTreeSet<MemoryLocation>, reason: impl Into<String>) -> Self {
        Self {
            locations,
            reason: reason.into(),
        }
    }

    /// True when `loc` is in this alias set.
    #[must_use]
    pub fn may_alias(&self, loc: &MemoryLocation) -> bool {
        self.locations.contains(loc)
    }

    /// All variable names in this set.
    #[must_use]
    pub fn names(&self) -> BTreeSet<String> {
        self.locations.iter().map(|l| l.name.clone()).collect()
    }

    /// True when `name` appears as any location's variable name.
    #[must_use]
    pub fn contains_name(&self, name: &str) -> bool {
        self.locations.iter().any(|l| l.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_location_display_matches_python() {
        let local = MemoryLocation::new(MemoryLocationKind::Local, "x");
        assert_eq!(local.display(), "x");

        let upvar =
            MemoryLocation::with_qualifier(MemoryLocationKind::Upvar, "local_x", "caller_x");
        assert_eq!(upvar.display(), "upvar(caller_x -> local_x)");

        let global = MemoryLocation::new(MemoryLocationKind::Global, "g");
        assert_eq!(global.display(), "global(g)");

        let ns = MemoryLocation::with_qualifier(MemoryLocationKind::NamespaceVar, "var", "::foo");
        assert_eq!(ns.display(), "ns(::foo::var)");

        let ivar =
            MemoryLocation::with_qualifier(MemoryLocationKind::InstanceVar, "self_x", "MyClass");
        assert_eq!(ivar.display(), "ivar(MyClass::self_x)");

        let elem = MemoryLocation::with_qualifier(MemoryLocationKind::ArrayElement, "arr", "key");
        assert_eq!(elem.display(), "arr(key)");

        let unk = MemoryLocation::new(MemoryLocationKind::Unknown, "?");
        assert_eq!(unk.display(), "?(?)");
    }

    #[test]
    fn alias_set_may_alias_and_names() {
        let mut locs = BTreeSet::new();
        locs.insert(MemoryLocation::new(MemoryLocationKind::Local, "a"));
        locs.insert(MemoryLocation::new(MemoryLocationKind::Local, "b"));
        let set = AliasSet::new(locs, "upvar");
        assert!(set.may_alias(&MemoryLocation::new(MemoryLocationKind::Local, "a")));
        assert!(!set.may_alias(&MemoryLocation::new(MemoryLocationKind::Local, "c")));
        assert!(set.contains_name("a"));
        assert!(set.contains_name("b"));
        assert!(!set.contains_name("c"));
        let names = set.names();
        assert!(names.contains("a"));
        assert!(names.contains("b"));
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn memory_location_ordering_stable() {
        // BTreeSet ordering should be deterministic so AliasSet's
        // rendered output is reproducible across runs.
        let mut set = BTreeSet::new();
        set.insert(MemoryLocation::new(MemoryLocationKind::Local, "z"));
        set.insert(MemoryLocation::new(MemoryLocationKind::Local, "a"));
        set.insert(MemoryLocation::new(MemoryLocationKind::Global, "m"));
        let names: Vec<_> = set.iter().map(|l| l.name.clone()).collect();
        assert_eq!(names, vec!["a".to_string(), "z".to_string(), "m".to_string()]);
    }
}
