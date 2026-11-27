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

//! Root variable accesses by the invocation that evaluated them.
//!
//! The owner projections select accesses by temporal owner, independently of
//! source position, so an unindexed query reads every access in the document.
//! This index lists, per root-origin invocation offset, the accesses each
//! projection selects, in inventory order.

use super::{SourceCommandBindings, SourceVariableAccess, SourceVariableEvaluationOwner};
use std::collections::BTreeMap;
use std::sync::Arc;
use tcl_names::command::CommandAllocationSite;
use tcl_names::origin::SourceOriginId;

type Accesses = BTreeMap<u32, Vec<SourceVariableAccess>>;
type AccessAt = (u32, usize);

#[derive(Default)]
pub(super) struct VariableAccessOwnerIndex {
    direct_arguments: rustc_hash::FxHashMap<u32, Vec<AccessAt>>,
    argument_evaluation: rustc_hash::FxHashMap<u32, Vec<AccessAt>>,
    argument_or_expression_evaluation: rustc_hash::FxHashMap<u32, Vec<AccessAt>>,
}

pub(super) type VariableAccessOwnerCache =
    super::shared_inventory::InventoryIndexCache<Accesses, VariableAccessOwnerIndex>;

/// Which owner projection a query reads.
#[derive(Clone, Copy)]
pub(super) enum AccessOwnerProjection {
    DirectArguments,
    ArgumentEvaluation,
    ArgumentOrExpressionEvaluation,
}

impl AccessOwnerProjection {
    fn selects(self, owner: &SourceVariableEvaluationOwner, site: &CommandAllocationSite) -> bool {
        match self {
            Self::DirectArguments => owner.is_direct_arguments_of(site),
            Self::ArgumentEvaluation => owner.is_argument_evaluation_of(site),
            Self::ArgumentOrExpressionEvaluation => {
                owner.is_argument_evaluation_of(site) || owner.is_expression_evaluation_of(site)
            }
        }
    }
}

/// Every invocation site named by an owner, with whether an argument
/// evaluation or a native expression names it and whether it is the owner's
/// own (not an enclosing) argument evaluation.
fn owner_sites<'a>(
    owner: &'a SourceVariableEvaluationOwner,
    direct: bool,
    found: &mut Vec<(&'a CommandAllocationSite, bool, bool)>,
) {
    match owner {
        SourceVariableEvaluationOwner::InvocationArguments { invocation, parent } => {
            found.push((invocation, true, direct));
            if let Some(parent) = parent {
                owner_sites(parent, false, found);
            }
        }
        SourceVariableEvaluationOwner::NativeExpression { invocation, parent } => {
            found.push((invocation, false, false));
            if let Some(parent) = parent {
                owner_sites(parent, false, found);
            }
        }
        SourceVariableEvaluationOwner::InvocationBody { parent, .. } => {
            if let Some(parent) = parent {
                owner_sites(parent, false, found);
            }
        }
        SourceVariableEvaluationOwner::Alternatives(owners) => {
            for owner in owners {
                owner_sites(owner, direct, found);
            }
        }
        SourceVariableEvaluationOwner::Unspecified => {}
    }
}

impl VariableAccessOwnerIndex {
    fn build(accesses: &Accesses, origin: &Arc<SourceOriginId>) -> Self {
        fn add(rows: &mut rustc_hash::FxHashMap<u32, Vec<AccessAt>>, offset: u32, at: AccessAt) {
            let rows = rows.entry(offset).or_default();
            if rows.last() != Some(&at) {
                rows.push(at);
            }
        }
        let mut index = Self::default();
        let mut sites = Vec::new();
        for (key, row) in accesses {
            for (position, access) in row.iter().enumerate() {
                sites.clear();
                owner_sites(&access.owner, true, &mut sites);
                let at = (*key, position);
                for (site, argument, direct) in &sites {
                    if &site.source != origin {
                        continue;
                    }
                    if *direct {
                        add(&mut index.direct_arguments, site.offset, at);
                    }
                    if *argument {
                        add(&mut index.argument_evaluation, site.offset, at);
                    }
                    add(
                        &mut index.argument_or_expression_evaluation,
                        site.offset,
                        at,
                    );
                }
            }
        }
        index
    }
}

impl SourceCommandBindings {
    fn scanned_variable_accesses_owned_by(
        &self,
        source: &Arc<SourceOriginId>,
        offset: u32,
        projection: AccessOwnerProjection,
    ) -> Vec<SourceVariableAccess> {
        let site = CommandAllocationSite {
            source: Arc::clone(source),
            offset,
        };
        self.variable_accesses
            .values()
            .flatten()
            .filter(|access| projection.selects(&access.owner, &site))
            .cloned()
            .collect()
    }

    /// Root accesses the projection selects for the invocation at `offset`,
    /// in inventory order: exactly the accesses a scan of the whole inventory
    /// with the projection's owner test returns.
    pub(super) fn variable_accesses_owned_by(
        &self,
        offset: u32,
        projection: AccessOwnerProjection,
    ) -> Vec<SourceVariableAccess> {
        let Some(source) = &self.root_origin else {
            return Vec::new();
        };
        let Some(index) = self.derived_indexes.get().variable_access_owners.get(
            &self.variable_accesses,
            source,
            |accesses| VariableAccessOwnerIndex::build(accesses, source),
        ) else {
            return self.scanned_variable_accesses_owned_by(source, offset, projection);
        };
        let rows = match projection {
            AccessOwnerProjection::DirectArguments => &index.direct_arguments,
            AccessOwnerProjection::ArgumentEvaluation => &index.argument_evaluation,
            AccessOwnerProjection::ArgumentOrExpressionEvaluation => {
                &index.argument_or_expression_evaluation
            }
        };
        rows.get(&offset)
            .into_iter()
            .flatten()
            .filter_map(|(key, position)| self.variable_accesses.get(key)?.get(*position))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dialect-invariant; run under Tcl 9.0.
    #[test]
    fn the_index_selects_exactly_what_the_scan_selects() {
        let source = "proc p {a b} {\n    set v [expr {$a + $b}]\n    \
            if {$v > 1} { set v [string length $a$b] }\n    return $v\n}\n\
            set x 1\nset y [p $x [string trim $x]]\nputs \"$x [p $y $x]\"\n";
        let registry = tcl_registry::CommandRegistry::build_default();
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            crate::command_binding::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
                ..crate::command_binding::SourceAnalysisOptions::default()
            },
        );
        let origin = bindings.root_origin.clone().expect("root origin");
        let mut selected = 0;
        for projection in [
            AccessOwnerProjection::DirectArguments,
            AccessOwnerProjection::ArgumentEvaluation,
            AccessOwnerProjection::ArgumentOrExpressionEvaluation,
        ] {
            for offset in 0..=u32::try_from(source.len()).unwrap() {
                let scanned =
                    bindings.scanned_variable_accesses_owned_by(&origin, offset, projection);
                // The second query of an unchanged inventory reads the index.
                let _ = bindings.variable_accesses_owned_by(offset, projection);
                let indexed = bindings.variable_accesses_owned_by(offset, projection);
                assert_eq!(indexed, scanned, "offset {offset}");
                selected += scanned.len();
            }
        }
        assert!(selected > 0, "the fixture must record owned accesses");
    }
}
