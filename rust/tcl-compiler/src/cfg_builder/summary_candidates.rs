// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Possible summary keys with their complete retained original procedures.

use std::collections::HashMap;

use crate::ir::{Module, Procedure};
use tcl_registry::CommandRegistry;

pub(super) struct ProcedureSummaryCandidates<'a> {
    by_key: HashMap<String, Option<Vec<&'a Procedure>>>,
}

impl<'a> ProcedureSummaryCandidates<'a> {
    pub(super) fn for_module(module: &'a Module, registry: &CommandRegistry) -> Self {
        let owner = module
            .retained_source_bindings
            .as_ref()
            .filter(|owner| owner.matches_module(module, registry));
        let mut by_key = HashMap::new();
        for (qname, procedure) in module.procedures.display_entries() {
            let original = qname == &procedure.qualified_name
                && !module.redefined_procedures.contains(qname)
                && owner.is_some_and(|owner| owner.matches_original_procedure(procedure));
            for key in tcl_syntax::naming::key_suffixes(qname) {
                let candidates = by_key.entry(key).or_insert_with(|| Some(Vec::new()));
                if original {
                    if let Some(candidates) = candidates {
                        candidates.push(procedure);
                    }
                } else {
                    // An absent original member cannot leave a false unique winner.
                    *candidates = None;
                }
            }
        }
        Self { by_key }
    }

    pub(super) fn keys(&self) -> impl Iterator<Item = &str> {
        self.by_key.keys().map(String::as_str)
    }

    pub(super) fn original(&self, key: &str) -> Option<&[&'a Procedure]> {
        self.by_key
            .get(key)?
            .as_deref()
            .filter(|candidates| !candidates.is_empty())
    }

    /// Complete declaration agreement includes ordered parameters, defaults
    /// and the trailing variadic declaration through the original raw header.
    pub(super) fn agreed_signature(&self, key: &str) -> Option<&'a Procedure> {
        let candidates = self.original(key)?;
        let first = *candidates.first()?;
        candidates
            .iter()
            .all(|candidate| {
                candidate.params == first.params && candidate.params_raw == first.params_raw
            })
            .then_some(first)
    }

    pub(super) fn agreed_parameters(&self) -> HashMap<String, Vec<String>> {
        self.keys()
            .filter_map(|key| Some((key.to_owned(), self.agreed_signature(key)?.params.clone())))
            .collect()
    }
}
