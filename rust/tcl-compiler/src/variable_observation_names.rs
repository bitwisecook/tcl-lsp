// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Textual MAY protection, separate from resolved cells and Native observer state.

use std::collections::{BTreeSet, HashSet};
use std::hash::BuildHasher;

/// Readonly literal report-name membership, independent of storage ordering.
/// This interface supplies no interpreted lookup or variable-cell identity.
pub trait VariableObservationNameSet {
    /// Whether this text inventory contains the exact supplied report.
    fn contains_name(&self, name: &str) -> bool;
}

impl VariableObservationNameSet for BTreeSet<String> {
    fn contains_name(&self, name: &str) -> bool {
        self.contains(name)
    }
}

impl<S: BuildHasher> VariableObservationNameSet for HashSet<String, S> {
    fn contains_name(&self, name: &str) -> bool {
        self.contains(name)
    }
}

/// The independently selected text policy of an observation inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableProtectionPurpose {
    /// Module trace/deferred inventories store root-marker-free report keys.
    ModuleObservation,
    /// Mixed source guards retain exact keys as well as module report keys.
    MixedSourceProtection,
    /// A native-lowering ledger retains exact, one-root and rooted text aliases.
    /// Entered Native observers and unknown ledgers remain independent.
    NativeLoweringLedger,
}

/// The text aliases retained by a conditional write-summary producer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableWriteObservationPurpose {
    /// Global callback writes retain one-root-free names and array roots.
    DeferredGlobal,
    /// Outer-frame summaries retain the original and one-root-free spelling.
    OuterScope,
}

/// A module observation key, never an interpreted name or resolved cell.
/// Literal sigils, braces, interior colon runs and a remaining lone colon stay data.
#[must_use]
pub fn module_observation_key(mut name: &str) -> &str {
    while let Some(unrooted) = tcl_syntax::naming::unroot_rooted_key(name) {
        name = unrooted;
    }
    name
}

/// Whether this caller's text inventory may protect `name`.
/// Unknown input protects every name; a false result grants no Native absence,
/// callback-free state, body entry, physical identity or store erasure.
#[must_use]
pub fn variable_name_may_be_observed(
    names: &impl VariableObservationNameSet,
    name: &str,
    unknown: bool,
    purpose: VariableProtectionPurpose,
) -> bool {
    if unknown {
        return true;
    }
    match purpose {
        VariableProtectionPurpose::ModuleObservation => {
            names.contains_name(module_observation_key(name))
        }
        VariableProtectionPurpose::MixedSourceProtection => {
            names.contains_name(name) || names.contains_name(module_observation_key(name))
        }
        VariableProtectionPurpose::NativeLoweringLedger => {
            names.contains_name(name)
                || tcl_syntax::naming::unroot_rooted_key(name)
                    .is_some_and(|unrooted| names.contains_name(unrooted))
                || names.contains_name(&tcl_syntax::naming::root_unrooted_key(name))
        }
    }
}

/// Retain possible write names under the producer's explicit text purpose.
/// The empty variable name remains possible; report aliases do not identify cells.
pub fn record_variable_write_name(
    names: &mut BTreeSet<String>,
    name: &str,
    purpose: VariableWriteObservationPurpose,
) {
    let unrooted = tcl_syntax::naming::unroot_rooted_key(name).unwrap_or(name);
    match purpose {
        VariableWriteObservationPurpose::DeferredGlobal => {
            names.insert(unrooted.to_owned());
            if let Some((array, _)) = tcl_syntax::naming::split_element_ref(unrooted) {
                names.insert(array.to_owned());
            }
        }
        VariableWriteObservationPurpose::OuterScope => {
            if unrooted != name {
                names.insert(unrooted.to_owned());
            }
            names.insert(name.to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn module_protection_keeps_namespace_and_literal_name_data() {
        let protected = names(&["n::x", ":x", "$x", "", "{x}"]);
        for name in ["::n::x", "::::n::x", ":::x", "$x", "", "{x}"] {
            assert!(
                variable_name_may_be_observed(
                    &protected,
                    name,
                    false,
                    VariableProtectionPurpose::ModuleObservation
                ),
                "{name:?}"
            );
        }
        for name in ["::other::x", "x", "${x}", "n:::x"] {
            assert!(
                !variable_name_may_be_observed(
                    &protected,
                    name,
                    false,
                    VariableProtectionPurpose::ModuleObservation
                ),
                "{name:?}"
            );
        }
    }

    #[test]
    fn native_lowering_ledger_does_not_borrow_module_root_pair_policy() {
        let protected = names(&["x", "::n::x"]);
        for name in ["x", "::x", "n::x", "::n::x"] {
            assert!(variable_name_may_be_observed(
                &protected,
                name,
                false,
                VariableProtectionPurpose::NativeLoweringLedger
            ));
        }
        assert!(!variable_name_may_be_observed(
            &protected,
            "::::x",
            false,
            VariableProtectionPurpose::NativeLoweringLedger
        ));
        assert!(variable_name_may_be_observed(
            &protected,
            "::::x",
            false,
            VariableProtectionPurpose::ModuleObservation
        ));
    }

    #[test]
    fn deferred_array_and_empty_writes_remain_possible_without_reference_decoration() {
        let mut deferred = BTreeSet::new();
        for name in ["::arr(key)", ":::x", "", "$x", "arr(open"] {
            record_variable_write_name(
                &mut deferred,
                name,
                VariableWriteObservationPurpose::DeferredGlobal,
            );
        }
        assert_eq!(
            deferred,
            names(&["arr(key)", "arr", ":x", "", "$x", "arr(open"])
        );
        assert!(!deferred.contains("x"));
        let mut outer = BTreeSet::new();
        record_variable_write_name(
            &mut outer,
            "::n::arr(key)",
            VariableWriteObservationPurpose::OuterScope,
        );
        assert_eq!(outer, names(&["::n::arr(key)", "n::arr(key)"]));
        assert!(!outer.contains("n::arr"));
        let mut rooted_empty = BTreeSet::new();
        for name in ["::", ":::"] {
            record_variable_write_name(
                &mut rooted_empty,
                name,
                VariableWriteObservationPurpose::OuterScope,
            );
        }
        assert_eq!(rooted_empty, names(&["::", "", ":::", ":"]));
        assert!(!rooted_empty.contains("n"));
    }

    #[test]
    fn equal_text_reports_and_unknown_state_supply_no_cell_identity() {
        let protected = names(&["x"]);
        use crate::command_binding::SourceNamespaceKey;
        use crate::var_resolve::{VariableCellKey, VariableCellSet};
        let left = VariableCellKey::Namespace {
            identity: SourceNamespaceKey::authored("::left"),
            simple: tcl_core_types::NameBytes::from("x"),
        };
        let right = VariableCellKey::Namespace {
            identity: SourceNamespaceKey::authored("::right"),
            simple: tcl_core_types::NameBytes::from("x"),
        };
        let mut cells = VariableCellSet::default();
        cells.insert(left.clone());
        assert_ne!(left, right);
        assert!(cells.contains(&left));
        assert!(!cells.contains(&right));
        // The shared source label protects both reports without merging cells.
        assert!(variable_name_may_be_observed(
            &protected,
            "x",
            false,
            VariableProtectionPurpose::ModuleObservation
        ));
        for name in ["::left::x", "::right::x", "other(key)", "", "$x"] {
            assert!(variable_name_may_be_observed(
                &BTreeSet::new(),
                name,
                true,
                VariableProtectionPurpose::ModuleObservation
            ));
        }
    }
}
