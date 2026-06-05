//! BIG-IP object registry — Rust port of the Python
//! `core/bigip/registry` object/property specs (GAP-e).
//!
//! Mirrors the Python `BigipObjectSpec` / `BigipObjectKindSpec` /
//! `BigipPropertySpec` / `ValueKind` model. Each [`BigipObjectSpec`]
//! describes one tmsh object kind (its module, object-type words,
//! header keys) and the schema of its properties (value kind, enum
//! values, references to other kinds, list operators, defaults, …).
//!
//! The spec data is `&'static` const data generated from the
//! reconciled Python source of truth (the canonical `origin/main`
//! baseline); see `scripts/registry-audit/gen_bigip_rust.py`.

use std::collections::HashMap;

pub mod data;

/// Canonical property value-kind vocabulary. Mirrors Python `ValueKind`.
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
    /// Stable wire tag matching the Python `ValueKind` value
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

/// Resolution metadata for one BIG-IP object kind. Mirrors Python
/// `BigipObjectKindSpec`.
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

/// Property metadata for schema/validation-aware tooling. Mirrors
/// Python `BigipPropertySpec`.
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
    /// `None` when unset (Python empty string).
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

/// Complete registry metadata owned by one BIG-IP object kind. Mirrors
/// Python `BigipObjectSpec`.
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_builds_and_resolves() {
        let reg = BigipRegistry::build();
        assert!(
            reg.len() > 900,
            "expected ~992 object kinds, got {}",
            reg.len()
        );
        let ldap = reg.get("auth_ldap").expect("auth_ldap present");
        assert!(
            ldap.properties.len() > 20,
            "auth_ldap should carry rich properties, got {}",
            ldap.properties.len()
        );
        // The reconciled rich data carries defaults / usage flags.
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
}
