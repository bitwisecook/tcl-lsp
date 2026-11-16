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

//! Normal handler transfer and stores: frozen targets, alias destinations and refused licences.

use super::*;

#[test]
fn normal_handler_transfer_preserves_unknown_compiler_protocol() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let tokens = f5_source_tokens("set connection_value 17");
    assert!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .compiled_execution_unknown()
    );
    assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());
    let normal = normal_transfer_invocation(registry, None, &tokens).unwrap();
    assert_eq!(normal.definition_names(), ["connection_value"]);
    assert_eq!(normal.written_argument(1), Some(1));
    let selected = &tokens.source_binding.as_ref().unwrap().variable_context;
    assert_eq!(
        normal.stored_value_literal(selected, registry).as_deref(),
        Some("17")
    );
    let mut observed = selected.as_ref().clone();
    observed.dynamic_traces = true;
    assert!(normal.stored_value_literal(&observed, registry).is_none());
    let statement = crate::ir::Statement::Call {
        span: tokens.words().first().unwrap().source().span,
        command: "set".into(),
        canonical_command: None,
        args: vec!["connection_value".into(), "17".into()],
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        foreach_groups: None,
        tokens: Some(tokens),
    };
    assert_eq!(
        crate::ssa::defs_of_with_registry(&statement, Some(registry)),
        ["connection_value"]
    );
    assert!(resolved_statement_invocation(registry, None, &statement).is_none());
}

#[test]
fn normal_store_uses_frozen_target_and_post_argv_observers() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let tokens = f5_source_tokens("set n x; set x OLD; set $n [set n NEW]");
    let normal = normal_transfer_invocation(registry, None, &tokens).unwrap();
    let selected = &tokens.source_binding.as_ref().unwrap().variable_context;
    assert_eq!(normal.definition_names(), ["x"]);
    assert_eq!(
        normal.stored_value_literal(selected, registry).as_deref(),
        Some("NEW")
    );
    assert!(resolved_tokens_invocation(registry, None, &tokens).is_none());

    let source = "proc observer {args} {set ::x CHANGED}; set x OLD; set x [trace add variable x write observer]";
    let tokens = f5_source_tokens(source);
    assert!(normal_transfer_invocation(registry, None, &tokens).is_none());
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let tokens = last_source_tokens(source);
    let normal = normal_transfer_invocation(registry, None, &tokens).unwrap();
    let selected = &tokens.source_binding.as_ref().unwrap().variable_context;
    assert!(normal.stored_value_operand(selected, registry).is_none());
    assert!(normal.stored_value_literal(selected, registry).is_none());
    assert!(resolved_tokens_invocation(registry, None, &tokens).is_some());
}

#[test]
fn normal_store_resolves_actual_alias_destination_after_arguments() {
    let source = "set x OLD; set y NEW; upvar 0 x a; set a [upvar 0 y a]";
    for dialect in ["f5-irules", "tcl8.6"] {
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let tokens = if dialect == "f5-irules" {
            f5_source_tokens(source)
        } else {
            last_source_tokens(source)
        };
        let normal = normal_transfer_invocation(registry, None, &tokens).unwrap();
        let selected = &tokens.source_binding.as_ref().unwrap().variable_context;
        let places = normal.mutation_places(selected, registry);
        assert_eq!(places.len(), 1);
        let target = crate::var_resolve::resolve_place("y", selected, false, registry);
        assert_eq!(places[0].kind, crate::place::PlaceKind::Scalar);
        assert!(!places[0].observed && !places[0].dynamic);
        assert!(places[0].cell.is_some());
        assert_eq!(places[0].cell, target.cell);
        assert_eq!(
            crate::var_resolve::canonical_binding_value_name(&places[0]),
            Some("::y".to_owned())
        );
        assert_eq!(
            normal.stored_value_literal(selected, registry).as_deref(),
            Some("")
        );
        assert_eq!(
            resolved_tokens_invocation(registry, None, &tokens).is_some(),
            dialect == "tcl8.6"
        );
    }
}

#[test]
fn normal_handler_transfer_rejects_mutation_and_body_licences() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    for source in [
        "set x [proc set args {return CUSTOM}]",
        "if {$flag} {set x 1}",
        "proc set args {return CUSTOM}; set x 1",
        "set x 1 extra",
    ] {
        let tokens = f5_source_tokens(source);
        assert!(
            normal_transfer_invocation(registry, None, &tokens).is_none(),
            "{source}"
        );
    }
}
