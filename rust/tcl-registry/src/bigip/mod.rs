// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! BIG-IP object registry — object/property specs.
//!
//! Each [`BigipObjectSpec`]
//! describes one tmsh object kind (its module, object-type words,
//! header keys) and the schema of its properties (value kind, enum
//! values, references to other kinds, list operators, defaults, …).
//!
//! The spec data (`data::a` .. `data::w`) is hand-maintained `&'static` const
//! data; see `data/mod.rs` and issue #1404 for provenance and the
//! `cargo xtask bigip-data-schema --check` consistency gate.

use std::collections::HashMap;

pub mod data;
mod references;

pub use references::REFERENCE_EDGES;

/// A config-level reference edge of the BIG-IP object graph: property
/// [`property`](ReferenceEdge::property) of [`from_kind`](ReferenceEdge::from_kind)
/// — optionally scoped to a [`section`](ReferenceEdge::section) — may name any of
/// [`to_kinds`](ReferenceEdge::to_kinds). Kind names are the registry's canonical
/// underscore form (`ltm_pool`, `gtm_monitor_http`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceEdge {
    /// Source object kind (canonical underscore form).
    pub from_kind: &'static str,
    /// The property that carries the reference.
    pub property: &'static str,
    /// Sub-section the property sits in, when the reference is nested.
    pub section: Option<&'static str>,
    /// Candidate target kinds the property may name.
    pub to_kinds: &'static [&'static str],
}

/// Reference edges declared for `from_kind`, in table order.
pub fn reference_edges_from(from_kind: &str) -> impl Iterator<Item = &'static ReferenceEdge> + '_ {
    REFERENCE_EDGES
        .iter()
        .filter(move |e| e.from_kind == from_kind)
}

/// The candidate target kinds `property` of `from_kind` may name — the first
/// matching edge's targets, or empty when there is none.
#[must_use]
pub fn reference_targets(from_kind: &str, property: &str) -> &'static [&'static str] {
    REFERENCE_EDGES
        .iter()
        .find(|e| e.from_kind == from_kind && e.property == property)
        .map_or(&[][..], |e| e.to_kinds)
}

/// Canonical property value-kind vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueKind {
    /// `string`.
    String,
    /// `integer`.
    Integer,
    /// `float`.
    Float,
    /// `boolean`.
    Boolean,
    /// `enum` — value drawn from [`BigipPropertySpec::enum_values`].
    Enum,
    /// `reference` — names another object kind.
    Reference,
    /// `list`.
    List,
    /// `block` — nests sub-properties.
    Block,
    /// `unknown` / unclassified.
    Unknown,
    /// `ip-address`.
    IpAddress,
    /// `endpoint`.
    Endpoint,
    /// `object`.
    Object,
}

impl ValueKind {
    /// Stable wire tag matching `ValueKind` value
    /// (`"string"`, `"ip-address"`, …) — used by the audit dumper.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::Boolean => "boolean",
            Self::Enum => "enum",
            Self::Reference => "reference",
            Self::List => "list",
            Self::Block => "block",
            Self::Unknown => "unknown",
            Self::IpAddress => "ip-address",
            Self::Endpoint => "endpoint",
            Self::Object => "object",
        }
    }
}

/// Resolution metadata for one BIG-IP object kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BigipObjectKindSpec {
    /// The registry's canonical kind name (e.g. `"ltm_virtual"`).
    pub kind: &'static str,
    /// Optional `BigipConfig` attribute name storing this kind.
    pub table_name: Option<&'static str>,
    /// Optional `BigipConfig` method name used to resolve references.
    pub resolver_name: Option<&'static str>,
    /// The tmsh module word (`ltm`, `gtm`, …).
    pub module: Option<&'static str>,
    /// tmsh object-type word(s) after the module (e.g. `"profile tcp"`).
    pub object_types: &'static [&'static str],
}

/// Property metadata for schema/validation-aware tooling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BigipPropertySpec {
    /// Property identifier as it appears in tmsh.
    pub name: &'static str,
    /// Collapsed value tag.
    pub value_type: ValueKind,
    /// Parent sections this property may appear in.
    pub in_sections: &'static [&'static str],
    /// tmsh requires this field at create time.
    pub required: bool,
    /// Property may appear multiple times.
    pub repeated: bool,
    /// The literal `none` clears the value.
    pub allow_none: bool,
    /// Permitted value-space members (for `enum`).
    pub enum_values: &'static [&'static str],
    /// Inclusive lower bound.
    pub min_value: Option<f64>,
    /// Inclusive upper bound.
    pub max_value: Option<f64>,
    /// Optional regex constraint.
    pub pattern: &'static str,
    /// Object kinds this property may name — outbound graph edges.
    pub references: &'static [&'static str],
    /// Human-readable description.
    pub description: &'static str,
    /// Richer shape kind when `value_type` collapses the scalar.
    /// `None` when unset.
    pub shape_kind: Option<ValueKind>,
    /// Documented default value.
    pub default: Option<&'static str>,
    /// Lifecycle flags (`deprecated`, `read_only`, `not_synced`, …).
    pub usage_flags: &'static [&'static str],
    /// tmsh list operators (`add` / `delete` / `replace-all-with`).
    pub list_operators: &'static [&'static str],
    /// Nested sub-properties for object-shaped blocks.
    pub block: &'static [BigipPropertySpec],
}

impl BigipPropertySpec {
    /// Const default — use with `..BigipPropertySpec::DEFAULT`.
    pub const DEFAULT: Self = Self {
        name: "",
        value_type: ValueKind::String,
        in_sections: &[],
        required: false,
        repeated: false,
        allow_none: false,
        enum_values: &[],
        min_value: None,
        max_value: None,
        pattern: "",
        references: &[],
        description: "",
        shape_kind: None,
        default: None,
        usage_flags: &[],
        list_operators: &[],
        block: &[],
    };

    /// True when this property is a tmsh list (operator required).
    #[must_use]
    pub const fn is_list_valued(&self) -> bool {
        !self.list_operators.is_empty()
    }

    /// True when this property nests sub-properties as a block.
    #[must_use]
    pub const fn is_block(&self) -> bool {
        !self.block.is_empty()
    }
}

/// Complete registry metadata owned by one BIG-IP object kind.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BigipObjectSpec {
    /// Identity record.
    pub kind_spec: BigipObjectKindSpec,
    /// `(module, object-type)` tuples the parser keys this kind by.
    pub header_types: &'static [(&'static str, &'static str)],
    /// Property specs declared on this object kind.
    pub properties: &'static [BigipPropertySpec],
}

impl BigipObjectSpec {
    /// The canonical kind name.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        self.kind_spec.kind
    }
}

/// Lookup facade over the BIG-IP object specs.
#[derive(Debug, Clone)]
pub struct BigipRegistry {
    specs: Vec<&'static BigipObjectSpec>,
    by_kind: HashMap<&'static str, usize>,
    by_header: HashMap<(&'static str, &'static str), usize>,
}

impl BigipRegistry {
    /// Build the registry from the generated `&'static` spec data.
    #[must_use]
    pub fn build() -> Self {
        let specs: Vec<&'static BigipObjectSpec> = data::all_specs();
        let mut by_kind = HashMap::with_capacity(specs.len());
        let mut by_header = HashMap::new();
        for (i, spec) in specs.iter().enumerate() {
            by_kind.insert(spec.kind_spec.kind, i);
            for &h in spec.header_types {
                by_header.insert(h, i);
            }
        }
        Self {
            specs,
            by_kind,
            by_header,
        }
    }

    /// Number of registered object kinds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.specs.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }

    /// All object specs, in registration order.
    #[must_use]
    pub fn specs(&self) -> &[&'static BigipObjectSpec] {
        &self.specs
    }

    /// Look up an object spec by its canonical kind name.
    #[must_use]
    pub fn get(&self, kind: &str) -> Option<&'static BigipObjectSpec> {
        self.by_kind.get(kind).map(|&i| self.specs[i])
    }

    /// Look up an object spec by a `(module, object-type)` header key.
    #[must_use]
    pub fn get_by_header(
        &self,
        module: &str,
        object_type: &str,
    ) -> Option<&'static BigipObjectSpec> {
        self.by_header
            .get(&(module, object_type))
            .map(|&i| self.specs[i])
    }

    /// All registered kind names, sorted.
    #[must_use]
    pub fn kind_names(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = self.by_kind.keys().copied().collect();
        names.sort_unstable();
        names
    }

    // Object-registry query layer: the kind / candidate-kind lookup
    // functions. These are the registry-backed
    // half of the BIG-IP reference graph; the config-dependent resolution
    // (`resolve_kind_in_configs`) and the graph builder itself live in
    // `tcl-bigip`, which owns `BigipConfig`.

    /// Property specs declared for `name` on the `(module, object_type)`
    /// container. Empty when the
    /// container is unknown or declares no such property.
    fn property_specs_for_context(
        &self,
        name: &str,
        module: Option<&str>,
        object_type: Option<&str>,
    ) -> Vec<&'static BigipPropertySpec> {
        let (Some(m), Some(ot)) = (module, object_type) else {
            return Vec::new();
        };
        let Some(spec) = self.get_by_header(m, ot) else {
            return Vec::new();
        };
        spec.properties.iter().filter(|p| p.name == name).collect()
    }

    /// Collect the referenced kinds from `property_specs`, restricted to those
    /// matching `section` and prioritising section-constrained properties
    fn kinds_from_properties(
        property_specs: &[&'static BigipPropertySpec],
        section: Option<&str>,
    ) -> Vec<&'static str> {
        let mut matching: Vec<&'static BigipPropertySpec> = property_specs
            .iter()
            .copied()
            .filter(|p| p.matches_section(section))
            .collect();
        // Stable sort: section-constrained properties (non-empty `in_sections`)
        // before generic ones.
        matching.sort_by_key(|p| usize::from(p.in_sections.is_empty()));
        let mut kinds: Vec<&'static str> = Vec::new();
        for prop in matching {
            for &kind in prop.references {
                if !kinds.contains(&kind) {
                    kinds.push(kind);
                }
            }
        }
        kinds
    }

    /// Candidate object kinds a property `key` may reference on the given
    /// container.
    #[must_use]
    pub fn candidate_kinds_for_key(
        &self,
        key: &str,
        section: Option<&str>,
        container_module: Option<&str>,
        container_object_type: Option<&str>,
    ) -> Vec<&'static str> {
        let specs = self.property_specs_for_context(key, container_module, container_object_type);
        Self::kinds_from_properties(&specs, section)
    }

    /// Candidate object kinds the items of a list `section` may reference
    #[must_use]
    pub fn candidate_kinds_for_section_item(
        &self,
        section: &str,
        container_module: Option<&str>,
        container_object_type: Option<&str>,
    ) -> Vec<&'static str> {
        let specs =
            self.property_specs_for_context(section, container_module, container_object_type);
        Self::kinds_from_properties(&specs, Some(section))
    }

    /// The canonical kind for a `(module, object_type)` stanza header, or
    /// `None` when unregistered.
    #[must_use]
    pub fn kind_for_header(&self, module: &str, object_type: &str) -> Option<&'static str> {
        self.get_by_header(module, object_type)
            .map(|spec| spec.kind_spec.kind)
    }

    /// Map a `Reference.target_kind` display string (`"ltm pool"`, `"ltm
    /// monitor"`, …) to registry-key kinds.
    ///
    /// An exact `(module, object_type)` header gives a single kind; a bare
    /// family prefix (`"ltm monitor"`) fans out to every matching specific kind
    /// in registration order (so a downstream resolver can try each).
    #[must_use]
    pub fn candidate_registry_kinds_for_display(&self, display_kind: &str) -> Vec<&'static str> {
        if display_kind.is_empty() {
            return Vec::new();
        }
        let (module, object_type) = display_kind.split_once(' ').unwrap_or((display_kind, ""));
        if let Some(spec) = self.get_by_header(module, object_type) {
            return vec![spec.kind_spec.kind];
        }
        let prefix = if object_type.is_empty() {
            String::new()
        } else {
            format!("{object_type} ")
        };
        // Iterate specs (and their header types) in registration order — the
        // same order `HEADER_KIND_MAP.items()` yields.
        let mut matches = Vec::new();
        for spec in &self.specs {
            for &(mod_, obj) in spec.header_types {
                if mod_ != module {
                    continue;
                }
                if !prefix.is_empty() && !obj.starts_with(&prefix) {
                    continue;
                }
                if prefix.is_empty() && obj.contains(' ') {
                    continue;
                }
                matches.push(spec.kind_spec.kind);
            }
        }
        matches
    }
}

impl BigipPropertySpec {
    /// Whether this property is in scope for `section`: an unconstrained
    /// property matches
    /// any section; otherwise `section` (defaulting to `""`) must be listed.
    #[must_use]
    pub fn matches_section(&self, section: Option<&str>) -> bool {
        if self.in_sections.is_empty() {
            return true;
        }
        self.in_sections.contains(&section.unwrap_or(""))
    }
}

/// The process-wide default BIG-IP object registry (built once on first use).
#[must_use]
pub fn default_registry() -> &'static BigipRegistry {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<BigipRegistry> = OnceLock::new();
    REGISTRY.get_or_init(BigipRegistry::build)
}

/// Sparse explicit BIG-IP version knowledge for the config schema — the
/// enrichment point for `(kind, property)` version ranges. The schema's
/// ~13k spec literals are declared against the BIG-IP 15.0 axis baseline
/// (`VersionKey::BigipVersion.baseline_version()`), so only entries whose
/// introduction/removal release is explicitly known appear here; a
/// property key of `""` records the object kind itself. Query through
/// [`BigipRegistry::object_version_range`] /
/// [`BigipRegistry::property_version_range`], never by reading this table
/// directly.
const BIGIP_SCHEMA_VERSIONS: &[(&str, &str, Option<&str>, Option<&str>)] = &[];

impl BigipRegistry {
    /// The declared BIG-IP version range of object `kind`: explicit
    /// knowledge from the sparse overlay, else the 15.0 axis baseline
    /// with an open maximum.
    #[must_use]
    pub fn object_version_range(kind: &str) -> (Option<&'static str>, Option<&'static str>) {
        Self::schema_version_range(kind, "")
    }

    /// The declared BIG-IP version range of `property` on object `kind`
    /// (baseline semantics as [`Self::object_version_range`]).
    #[must_use]
    pub fn property_version_range(
        kind: &str,
        property: &str,
    ) -> (Option<&'static str>, Option<&'static str>) {
        Self::schema_version_range(kind, property)
    }

    fn schema_version_range(
        kind: &str,
        property: &str,
    ) -> (Option<&'static str>, Option<&'static str>) {
        BIGIP_SCHEMA_VERSIONS
            .iter()
            .find(|(k, p, _, _)| *k == kind && *p == property)
            .map_or(
                (
                    tcl_dialect::VersionKey::BigipVersion.baseline_version(),
                    None,
                ),
                |&(_, _, min, max)| (min, max),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_builds_and_resolves() {
        let reg = BigipRegistry::build();
        // The generated object-spec data carries 798 object kinds.
        assert_eq!(
            reg.len(),
            798,
            "expected 798 object kinds, got {}",
            reg.len()
        );
        let ldap = reg.get("auth_ldap").expect("auth_ldap present");
        assert!(
            ldap.properties.len() > 20,
            "auth_ldap should carry rich properties, got {}",
            ldap.properties.len()
        );
        // The rich object data carries defaults / usage flags.
        let bind_dn = ldap
            .properties
            .iter()
            .find(|p| p.name == "bind-dn")
            .expect("bind-dn property");
        assert!(bind_dn.allow_none);
    }

    #[test]
    fn value_kind_tags_round_trip() {
        assert_eq!(ValueKind::IpAddress.as_str(), "ip-address");
        assert_eq!(ValueKind::Reference.as_str(), "reference");
    }

    #[test]
    fn kind_for_header_resolves_known_stanzas_and_round_trips() {
        let reg = default_registry();
        // A known stanza header resolves to a kind that maps back to a spec
        // carrying that same (module, object_type) header.
        let pool_kind = reg
            .kind_for_header("ltm", "pool")
            .expect("`ltm pool` is registered");
        let spec = reg.get(pool_kind).expect("kind round-trips to a spec");
        assert!(
            spec.header_types
                .iter()
                .any(|&(m, o)| m == "ltm" && o == "pool"),
            "the resolved kind's spec carries the ltm/pool header"
        );
        // An unregistered header resolves to nothing.
        assert!(reg.kind_for_header("ltm", "__not_a_real_type__").is_none());
        // The module disambiguates same-named object types across families.
        if let Some(gtm_pool) = reg.kind_for_header("gtm", "pool") {
            assert_ne!(
                pool_kind, gtm_pool,
                "ltm pool and gtm pool are distinct kinds"
            );
        }
    }

    #[test]
    fn candidate_registry_kinds_for_display_matches_header_and_fans_out() {
        let reg = default_registry();
        // An empty display string yields no candidates.
        assert!(reg.candidate_registry_kinds_for_display("").is_empty());
        // An exact `module object_type` display yields exactly the header kind.
        let by_display = reg.candidate_registry_kinds_for_display("ltm pool");
        let by_header = reg.kind_for_header("ltm", "pool").expect("`ltm pool`");
        assert_eq!(by_display, vec![by_header]);
        // A bare family prefix (`ltm monitor`) fans out to its specific subtypes,
        // each of which round-trips to a real spec.
        let monitors = reg.candidate_registry_kinds_for_display("ltm monitor");
        assert!(!monitors.is_empty(), "`ltm monitor` fans out to subtypes");
        for kind in &monitors {
            assert!(
                reg.get(kind).is_some(),
                "fanned-out kind {kind} round-trips"
            );
        }
    }

    #[test]
    fn reference_edges_are_broad_and_well_formed() {
        assert!(
            REFERENCE_EDGES.len() > 500,
            "expected a broad reference-edge set, got {}",
            REFERENCE_EDGES.len()
        );
        // Every edge names at least one target.
        assert!(REFERENCE_EDGES.iter().all(|e| !e.to_kinds.is_empty()));
        // A canonical edge: an ltm pool's members name ltm nodes.
        assert_eq!(reference_targets("ltm_pool", "members"), &["ltm_node"]);
        assert!(reference_edges_from("ltm_pool").count() >= 1);
        // An unknown source/property yields no targets.
        assert!(reference_targets("ltm_pool", "__no_such_prop__").is_empty());
        assert_eq!(reference_edges_from("__no_such_kind__").count(), 0);
    }

    #[test]
    fn reference_edge_source_kinds_mostly_resolve() {
        // Reference-edge source kinds use the registry's canonical kind naming,
        // so the vast majority resolve to a registered object spec.
        let reg = default_registry();
        let sources: std::collections::HashSet<&str> =
            REFERENCE_EDGES.iter().map(|e| e.from_kind).collect();
        let resolved = sources.iter().filter(|k| reg.get(k).is_some()).count();
        assert!(
            resolved * 100 >= sources.len() * 80,
            "expected ≥80% of edge source kinds to resolve, got {resolved}/{}",
            sources.len()
        );
    }
}
