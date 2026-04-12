//! Build a `name → literal` map from SSA versions + SCCP lattice.
//!
//! Ported from `core/compiler/optimiser/_helpers.py` —
//! `_constants_from_uses` and `_constants_from_exit_versions`.
//! Both take the same lattice map and build the same result
//! shape; only the input `name → version` dict differs (one comes
//! from a statement's `uses`, the other from a block's
//! `exit_versions`).

#![allow(clippy::implicit_hasher)]

use std::collections::HashMap;

use crate::analyses::LatticeValue;
use crate::ssa::ValueKey;

use super::literals::format_constant;

/// Map a `name → ssa_version` dictionary to a `name → Tcl source
/// literal` dictionary by consulting the SCCP `values` lattice.
///
/// Only `LatticeValue::Const` entries are materialised; non-const
/// lattice values (`Unknown` / `ConstSet` / `Overdefined`) and
/// zero-versioned entries (placeholder for "no definition seen
/// yet") are skipped.
#[must_use]
pub fn constants_from_versions(
    versions: &HashMap<String, u32>,
    values: &HashMap<ValueKey, LatticeValue>,
) -> HashMap<String, String> {
    let mut out: HashMap<String, String> = HashMap::new();
    for (name, ver) in versions {
        if *ver == 0 {
            continue;
        }
        let Some(lv) = values.get(&(name.clone(), *ver)) else {
            continue;
        };
        let LatticeValue::Const(cv) = lv else {
            continue;
        };
        if let Some(s) = format_constant(cv) {
            out.insert(name.clone(), s);
        }
    }
    out
}

/// Thin alias matching the Python `_constants_from_uses` name —
/// the implementation is identical to
/// [`constants_from_versions`]; keep both spellings so callers
/// reading one-line Python ports still find the expected symbol.
#[must_use]
pub fn constants_from_uses(
    uses: &HashMap<String, u32>,
    values: &HashMap<ValueKey, LatticeValue>,
) -> HashMap<String, String> {
    constants_from_versions(uses, values)
}

/// Thin alias matching `_constants_from_exit_versions`.
#[must_use]
pub fn constants_from_exit_versions(
    exit_versions: &HashMap<String, u32>,
    values: &HashMap<ValueKey, LatticeValue>,
) -> HashMap<String, String> {
    constants_from_versions(exit_versions, values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::ConstValue;

    fn lat_map(entries: &[(&str, u32, LatticeValue)]) -> HashMap<ValueKey, LatticeValue> {
        entries
            .iter()
            .map(|(n, v, lv)| (((*n).to_string(), *v), lv.clone()))
            .collect()
    }

    #[test]
    fn const_int_materialises_to_decimal_string() {
        let versions: HashMap<String, u32> = [("x".into(), 1_u32)].into_iter().collect();
        let values = lat_map(&[("x", 1, LatticeValue::Const(ConstValue::Int(42)))]);
        let out = constants_from_versions(&versions, &values);
        assert_eq!(out.get("x").map(String::as_str), Some("42"));
    }

    #[test]
    fn non_const_lattice_values_are_skipped() {
        let versions: HashMap<String, u32> = [
            ("known".into(), 1_u32),
            ("uk".into(), 1_u32),
            ("od".into(), 1_u32),
            ("cs".into(), 1_u32),
        ]
        .into_iter()
        .collect();
        let values = lat_map(&[
            ("known", 1, LatticeValue::Const(ConstValue::Int(7))),
            ("uk", 1, LatticeValue::Unknown),
            ("od", 1, LatticeValue::Overdefined),
            (
                "cs",
                1,
                LatticeValue::ConstSet(vec![ConstValue::Int(1), ConstValue::Int(2)]),
            ),
        ]);
        let out = constants_from_versions(&versions, &values);
        assert_eq!(out.len(), 1);
        assert!(out.contains_key("known"));
    }

    #[test]
    fn version_zero_and_missing_lattice_skipped() {
        let versions: HashMap<String, u32> =
            [("a".into(), 0_u32), ("b".into(), 1_u32)].into_iter().collect();
        // Only `b` has a lattice entry — and `a` would be skipped
        // even if it did, because its version is 0.
        let values = lat_map(&[("b", 1, LatticeValue::Const(ConstValue::Int(1)))]);
        let out = constants_from_versions(&versions, &values);
        assert_eq!(out.len(), 1);
        assert!(out.contains_key("b"));
    }

    #[test]
    fn aliases_share_implementation() {
        let versions: HashMap<String, u32> = [("x".into(), 1_u32)].into_iter().collect();
        let values = lat_map(&[("x", 1, LatticeValue::Const(ConstValue::Int(3)))]);
        assert_eq!(
            constants_from_uses(&versions, &values),
            constants_from_exit_versions(&versions, &values),
        );
    }
}
