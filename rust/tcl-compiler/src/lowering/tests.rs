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

use crate::interprocedural::test_body_identity::{fixture_id, fixture_id_if_present};

mod body_units;
mod class_members;
mod command_lowering;
mod configuration;
mod const_map;
mod dialect_grammar;
mod irules_events;
mod namespace_directives;
mod nesting_depth;
mod procedures;
mod static_eval;
mod structured_hooks;
mod trace_facts;

fn fixture_method<'a>(
    methods: &'a crate::ir::MethodInventory,
    label: &str,
) -> Option<&'a crate::ir::MethodDef> {
    methods.unique_display(label).map(|(_, method)| method)
}

/// An opaque invocation retains its original evaluated argv without licensing
/// native instructions, a known normal store, or an invented entered body.
pub(super) fn assert_runtime_opaque_call(
    statement: &crate::ir::Statement,
    registry: &tcl_registry::CommandRegistry,
    expected_head: &str,
) {
    let crate::ir::Statement::Call {
        command,
        args,
        tokens: Some(tokens),
        ..
    } = statement
    else {
        panic!("expected retained invocation, got {statement:?}");
    };
    assert_eq!(command, expected_head);
    assert_eq!(tokens.words().len(), args.len() + 1);
    assert!(tokens.source_binding.is_some(), "missing dispatch receipt");
    assert!(crate::registry_invocation::proved_native_inline_operation(tokens).is_none());
    if let Some(region) = tokens.evaluated_body() {
        assert!(
            region.possible_bodies.is_some(),
            "runtime retains an analysis-only body inventory"
        );
        assert_eq!(
            region.selection,
            crate::execution_region::RegionSelection::MaySkip
        );
        assert_eq!(
            region.residual_effects,
            crate::execution_region::WrapperEffectProjection::OpaqueResidual
        );
    }
    let context = &tokens.source_binding.as_ref().unwrap().variable_context;
    let mutations = crate::place_bridge::statement_mutation_places(statement, context, registry);
    assert!(
        mutations
            .iter()
            .any(|place| place.kind == tcl_names::place::PlaceKind::Unknown)
            || tokens.evaluated_body().is_some()
            || crate::registry_invocation::normal_transfer_invocation(registry, None, tokens)
                .is_none(),
        "a runtime body cannot grant bounded normal stores: {mutations:?}"
    );
}

/// Every C-Tcl dialect folds a brace-word line continuation; the Jim
/// answer is asserted in `jim_brace_continuation_binds_different_names`.
const FOLD_RULES: WordValueRules = WordValueRules::TCL;

/// `JimTcl`: the braced word keeps its bytes, and a malformed remainder is
/// split rather than raised.
const LITERAL_RULES: WordValueRules = WordValueRules::JIM;

use super::*;

fn reg() -> CommandRegistry {
    // These fixtures assert concrete C Tcl behaviour, including 9.1 grammar.
    CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
    )
}
